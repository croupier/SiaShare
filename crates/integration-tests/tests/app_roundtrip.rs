//! Сквозной цикл раздачи: команды и воркеры uploader-desktop загружают файлы
//! в MockNetwork, команды receiver-micro скачивают их по seed ключа.
//! Оба приложения живут в одном тестовом процессе (мок внутрипроцессный),
//! сетевых запросов нет. Запуск: cargo test -p integration-tests --offline

use receiver_micro::commands::{download_one, download_selected_zip, ReceiverState};
use sia_core::sia_client::fetch_shared_tree;
use sia_storage::mock::MockNetwork;
use sia_storage::{AppKey, Sdk};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;
use tauri::Manager;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use uploader_desktop::commands::{
    cleanup_session, revoke_session, start_upload_session, UploaderState,
};
use uploader_desktop::db::Database;
use uploader_desktop::worker::start_upload_workers;

const HOSTS: usize = 40;
const MIB: u64 = 1024 * 1024;

async fn mock_sdk() -> (MockNetwork, Sdk) {
    let net = MockNetwork::new();
    net.add_hosts(HOSTS);
    let sdk = net.sdk(AppKey::import([7u8; 32])).await.unwrap();
    (net, sdk)
}

async fn write_pattern_file(path: &Path, len: u64) {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.unwrap();
    }
    let mut f = tokio::fs::File::create(path).await.unwrap();
    let mut buf = vec![0u8; MIB as usize];
    let mut offset = 0u64;
    while offset < len {
        let n = (buf.len() as u64).min(len - offset) as usize;
        for (j, b) in buf[..n].iter_mut().enumerate() {
            *b = ((offset + j as u64) % 251) as u8;
        }
        f.write_all(&buf[..n]).await.unwrap();
        offset += n as u64;
    }
    f.flush().await.unwrap();
}

async fn file_blake3(path: &Path) -> String {
    let mut f = tokio::fs::File::open(path).await.unwrap();
    let mut h = blake3::Hasher::new();
    let mut buf = vec![0u8; MIB as usize];
    loop {
        let n = f.read(&mut buf).await.unwrap();
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    h.finalize().to_hex().to_string()
}

/// Uploader на мок-рантайме Tauri: реальные воркеры + БД, SDK подменен в watch-канале.
fn start_uploader(dir: &Path, sdk: Sdk) -> (tauri::App<tauri::test::MockRuntime>, Database) {
    let app = tauri::test::mock_app();
    let db = Database::init(&dir.join("queue.db")).unwrap();
    let sdk_tx = start_upload_workers(app.handle().clone(), db.clone(), 2);
    sdk_tx.send(Some(Arc::new(sdk))).unwrap();
    app.manage(UploaderState {
        db: db.clone(),
        sdk_tx,
    });
    (app, db)
}

async fn wait_status(db: &Database, sid: &str, want: &str) {
    tokio::time::timeout(Duration::from_secs(180), async {
        loop {
            if db.session_status(sid).unwrap().as_deref() == Some(want) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("сессия {sid} не достигла статуса '{want}' за 180 с"));
}

fn session_seed(db: &Database, sid: &str) -> [u8; 32] {
    let seed_hex = db.session_seed(sid).unwrap().expect("у сессии нет seed");
    hex::decode(seed_hex).unwrap().try_into().unwrap()
}

/// Receiver на мок-рантайме: состояние заполняется так же, это делает
/// load_files_with_key после connect_shared (сам connect в мок не идет).
fn start_receiver(
    shared: Arc<sia_storage::SharedSdk>,
    files: Vec<sia_core::models::SharedFileEntry>,
) -> tauri::App<tauri::test::MockRuntime> {
    let app = tauri::test::mock_app();
    app.manage(ReceiverState {
        encrypted_blob: Mutex::new(None),
        sdk: Mutex::new(Some(shared)),
        files: Mutex::new(files),
        cancel: Arc::new(AtomicBool::new(false)),
        download_gate: Mutex::new(()),
    });
    app
}

#[tokio::test]
async fn large_file_uploader_to_receiver() {
    let (net, sdk) = mock_sdk().await;
    let dir = tempfile::tempdir().unwrap();
    // 300 МиБ = 2 чанка реального размера (256 МиБ + 44 МиБ)
    let src = dir.path().join("large.bin");
    let total = 300 * MIB;
    write_pattern_file(&src, total).await;

    let (app, db) = start_uploader(dir.path(), sdk);
    let sid = start_upload_session(
        "large".into(),
        vec![src.to_string_lossy().into_owned()],
        0,
        app.state(),
    )
    .await
    .unwrap();
    wait_status(&db, &sid, "completed").await;

    let shared = net.shared_sdk(session_seed(&db, &sid)).await.unwrap();
    let tree = fetch_shared_tree(&shared).await.unwrap();
    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0].rel_path, "large.bin");
    assert_eq!(tree[0].total_size, total);
    assert_eq!(tree[0].chunk_object_ids.len(), 2);

    let rapp = start_receiver(Arc::new(shared), tree);
    let dest = dir.path().join("downloaded.bin");
    download_one(
        "large.bin".into(),
        dest.to_string_lossy().into_owned(),
        rapp.handle().clone(),
        rapp.state(),
    )
    .await
    .unwrap();
    assert_eq!(tokio::fs::metadata(&dest).await.unwrap().len(), total);
    assert_eq!(file_blake3(&dest).await, file_blake3(&src).await);
}

#[tokio::test]
async fn many_files_folder_uploader_to_receiver_zip() {
    let (net, sdk) = mock_sdk().await;
    let dir = tempfile::tempdir().unwrap();
    // 250 файлов + пустой, вложенные папки; 251 объект > PAGE=100 (пагинация)
    let root = dir.path().join("many");
    for i in 0..250u32 {
        let p = root.join(format!("sub{}/file{i:03}.bin", i % 10));
        write_pattern_file(&p, 128 * 1024).await;
    }
    write_pattern_file(&root.join("empty.dat"), 0).await;

    let (app, db) = start_uploader(dir.path(), sdk);
    let sid = start_upload_session(
        "many".into(),
        vec![root.to_string_lossy().into_owned()],
        7,
        app.state(),
    )
    .await
    .unwrap();
    wait_status(&db, &sid, "completed").await;

    let shared = net.shared_sdk(session_seed(&db, &sid)).await.unwrap();
    let tree = fetch_shared_tree(&shared).await.unwrap();
    assert_eq!(tree.len(), 251);

    let rapp = start_receiver(Arc::new(shared), tree.clone());
    let all_paths: Vec<String> = tree.iter().map(|e| e.rel_path.clone()).collect();
    let zip_path = dir.path().join("all.zip");
    download_selected_zip(
        all_paths,
        zip_path.to_string_lossy().into_owned(),
        rapp.handle().clone(),
        rapp.state(),
    )
    .await
    .unwrap();

    let mut archive = zip::ZipArchive::new(std::fs::File::open(&zip_path).unwrap()).unwrap();
    assert_eq!(archive.len(), 251);
    let expected: Vec<u8> = (0..128 * 1024).map(|j| (j % 251) as u8).collect();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).unwrap();
        let mut data = Vec::new();
        std::io::Read::read_to_end(&mut entry, &mut data).unwrap();
        if entry.name().ends_with("empty.dat") {
            assert!(data.is_empty());
        } else {
            assert_eq!(data, expected, "{}", entry.name());
        }
    }

    // Одиночное скачивание из той же раздачи (гейт скачиваний уже свободен)
    let one = tree
        .iter()
        .find(|e| e.rel_path.ends_with("file000.bin"))
        .unwrap();
    let dest = dir.path().join("one.bin");
    download_one(
        one.rel_path.clone(),
        dest.to_string_lossy().into_owned(),
        rapp.handle().clone(),
        rapp.state(),
    )
    .await
    .unwrap();
    assert_eq!(
        file_blake3(&dest).await,
        blake3::hash(&expected).to_hex().to_string()
    );
}

#[tokio::test]
async fn worker_retries_after_pin_failures_exhaust_sdk_retries() {
    let (net, sdk) = mock_sdk().await;
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("data.bin");
    write_pattern_file(&src, MIB).await;

    // 3 сбоя = все внутренние ретраи SDK исчерпаны → ошибка доходит до воркера,
    // он делает backoff и повторяет пачку (следующие пины уже успешны)
    net.set_pin_slabs_failures(3);

    let (app, db) = start_uploader(dir.path(), sdk);
    let sid = start_upload_session(
        "retry".into(),
        vec![src.to_string_lossy().into_owned()],
        0,
        app.state(),
    )
    .await
    .unwrap();
    wait_status(&db, &sid, "completed").await;
    assert!(net.pin_slabs_calls() >= 4);

    let shared = net.shared_sdk(session_seed(&db, &sid)).await.unwrap();
    let tree = fetch_shared_tree(&shared).await.unwrap();
    let rapp = start_receiver(Arc::new(shared), tree);
    let dest = dir.path().join("out.bin");
    download_one(
        "data.bin".into(),
        dest.to_string_lossy().into_owned(),
        rapp.handle().clone(),
        rapp.state(),
    )
    .await
    .unwrap();
    assert_eq!(file_blake3(&dest).await, file_blake3(&src).await);
}

#[tokio::test]
async fn revoke_then_cleanup_frees_quota() {
    let (net, sdk) = mock_sdk().await;
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("data.bin");
    write_pattern_file(&src, MIB).await;

    let (app, db) = start_uploader(dir.path(), sdk);
    let sid = start_upload_session(
        "quota".into(),
        vec![src.to_string_lossy().into_owned()],
        0,
        app.state(),
    )
    .await
    .unwrap();
    wait_status(&db, &sid, "completed").await;
    assert!(net.pinned_slabs() > 0);

    revoke_session(sid.clone(), app.state()).await.unwrap();
    assert_eq!(db.session_status(&sid).unwrap().as_deref(), Some("revoked"));

    cleanup_session(sid.clone(), app.state()).await.unwrap();
    assert_eq!(db.session_status(&sid).unwrap().as_deref(), Some("cleaned"));
    // объекты удалены + prune_slabs: квота мок-индексатора полностью освобождена
    assert_eq!(net.pinned_slabs(), 0);
}
