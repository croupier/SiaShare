//! Интеграционные тесты против встроенной мок-сети SDK (feature `mock`):
//! MockNetwork = индексатор + хосты в памяти, Sdk/SharedSdk настоящие,
//! erasure coding и шифрование выполняются по-настоящему, сетевых запросов нет.
//!
//! Тяжелый тест с реальным размером чанка (256 МиБ) помечен #[ignore]:
//! cargo test -p sia-core --test mock_network -- --ignored

use sia_core::downloader::download_single_file;
use sia_core::models::ChunkMetadata;
use sia_core::sia_client::{
    ensure_capacity, fetch_shared_tree, share_session_objects, upload_packed_chunks,
    ChunkUploadSpec, DEFAULT_CHUNK_SIZE,
};
use sia_storage::mock::MockNetwork;
use sia_storage::{AppKey, Hash256, Sdk};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const HOSTS: usize = 40;
const MIB: u64 = 1024 * 1024;

async fn mock_sdk() -> (MockNetwork, Sdk) {
    let net = MockNetwork::new();
    net.add_hosts(HOSTS);
    let sdk = net.sdk(AppKey::import([7u8; 32])).await.unwrap();
    (net, sdk)
}

/// Детерминированные данные: байт на смещении i = (i % 251). Позиционно-зависимый
/// узор ловит перестановку/дублирование чанков, а не только совпадение длины.
async fn write_pattern_file(path: &Path, len: u64) {
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

fn chunk_specs(local: &Path, rel: &str, total: u64, chunk_size: u64) -> Vec<ChunkUploadSpec> {
    let total_chunks = if total == 0 {
        1
    } else {
        total.div_ceil(chunk_size)
    } as u32;
    (0..total_chunks)
        .map(|idx| {
            let offset = idx as u64 * chunk_size;
            let length = if total == 0 {
                0
            } else {
                chunk_size.min(total - offset)
            };
            ChunkUploadSpec {
                local_path: local.to_string_lossy().into_owned(),
                offset_bytes: offset,
                length_bytes: length,
                meta: ChunkMetadata {
                    rel_path: rel.into(),
                    chunk_idx: idx,
                    total_chunks,
                    total_file_size: total,
                    chunk_size,
                    blake3: None,
                },
            }
        })
        .collect()
}

/// Загрузка + раздача; возвращает seed ключа для получателя.
async fn upload_and_share(sdk: &Sdk, specs: &[ChunkUploadSpec]) -> [u8; 32] {
    let objects = upload_packed_chunks(sdk, specs, |_| {}).await.unwrap();
    let ids: Vec<Hash256> = objects.iter().map(|o| o.id()).collect();
    share_session_objects(sdk, "test session", None, &ids, None)
        .await
        .unwrap()
}

fn no_cancel() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}

#[tokio::test]
async fn multi_chunk_roundtrip_with_blake3() {
    let (net, sdk) = mock_sdk().await;
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("big.bin");
    let total = 12 * MIB;
    write_pattern_file(&src, total).await;
    ensure_capacity(&sdk, total).await.unwrap();

    let specs = chunk_specs(&src, "folder/big.bin", total, MIB);
    let seed = upload_and_share(&sdk, &specs).await;

    let shared = net.shared_sdk(seed).await.unwrap();
    let tree = fetch_shared_tree(&shared).await.unwrap();
    assert_eq!(tree.len(), 1);
    let entry = &tree[0];
    assert_eq!(entry.rel_path, "folder/big.bin");
    assert_eq!(entry.total_size, total);
    assert_eq!(entry.chunk_object_ids.len(), 12);
    assert!(entry.chunk_blake3.iter().all(|h| h.is_some()));

    let dest = dir.path().join("out.bin");
    download_single_file(&shared, entry, &dest, no_cancel(), |_, _| {})
        .await
        .unwrap();
    assert_eq!(tokio::fs::metadata(&dest).await.unwrap().len(), total);
    assert_eq!(file_blake3(&dest).await, file_blake3(&src).await);
}

#[tokio::test]
async fn many_files_pagination_and_zip() {
    let (net, sdk) = mock_sdk().await;
    let dir = tempfile::tempdir().unwrap();
    // 150 объектов > PAGE=100 в fetch_shared_tree — обязательная пагинация
    let mut specs = Vec::new();
    for i in 0..150u32 {
        let p = dir.path().join(format!("f{i:03}.bin"));
        write_pattern_file(&p, 64 * 1024).await;
        specs.extend(chunk_specs(
            &p,
            &format!("dir{}/file{i:03}.bin", i / 50),
            64 * 1024,
            MIB,
        ));
    }
    let seed = upload_and_share(&sdk, &specs).await;

    let shared = net.shared_sdk(seed).await.unwrap();
    let tree = fetch_shared_tree(&shared).await.unwrap();
    assert_eq!(tree.len(), 150);
    let paths: Vec<&str> = tree.iter().map(|e| e.rel_path.as_str()).collect();
    let mut sorted = paths.clone();
    sorted.sort();
    assert_eq!(paths, sorted);

    // ZIP всех файлов получателя (путь receiver-micro)
    let zip_path = dir.path().join("all.zip");
    sia_core::downloader::download_as_zip(
        Arc::new(shared),
        tree,
        zip_path.clone(),
        no_cancel(),
        |_, _| {},
    )
    .await
    .unwrap();

    let zip_file = std::fs::File::open(&zip_path).unwrap();
    let mut archive = zip::ZipArchive::new(zip_file).unwrap();
    assert_eq!(archive.len(), 150);
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).unwrap();
        assert_eq!(entry.size(), 64 * 1024);
        let mut data = Vec::new();
        std::io::Read::read_to_end(&mut entry, &mut data).unwrap();
        let expected: Vec<u8> = (0..64 * 1024).map(|j| (j % 251) as u8).collect();
        assert_eq!(data, expected, "{}", entry.name());
    }
}

#[tokio::test]
async fn empty_file_roundtrip() {
    let (net, sdk) = mock_sdk().await;
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("empty.bin");
    write_pattern_file(&src, 0).await;

    let specs = chunk_specs(&src, "empty.bin", 0, MIB);
    let seed = upload_and_share(&sdk, &specs).await;

    let shared = net.shared_sdk(seed).await.unwrap();
    let tree = fetch_shared_tree(&shared).await.unwrap();
    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0].total_size, 0);

    let dest = dir.path().join("out.bin");
    download_single_file(&shared, &tree[0], &dest, no_cancel(), |_, _| {})
        .await
        .unwrap();
    assert_eq!(tokio::fs::metadata(&dest).await.unwrap().len(), 0);
}

#[tokio::test]
async fn resume_from_trusted_part() {
    let (net, sdk) = mock_sdk().await;
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("data.bin");
    let total = 12 * MIB;
    write_pattern_file(&src, total).await;
    let seed = upload_and_share(&sdk, &chunk_specs(&src, "data.bin", total, MIB)).await;

    let shared = net.shared_sdk(seed).await.unwrap();
    let tree = fetch_shared_tree(&shared).await.unwrap();
    let entry = &tree[0];

    // Имитация прерванной загрузки: первые 5 чанков уже в .part + валидный сайдкар
    let dest = dir.path().join("out.bin");
    let part = dir.path().join("out.bin.part");
    let mut src_f = tokio::fs::File::open(&src).await.unwrap();
    let mut part_f = tokio::fs::File::create(&part).await.unwrap();
    let mut copied = 0u64;
    let mut buf = vec![0u8; MIB as usize];
    while copied < 5 * MIB {
        let n = src_f.read(&mut buf).await.unwrap();
        part_f.write_all(&buf[..n]).await.unwrap();
        copied += n as u64;
    }
    part_f.flush().await.unwrap();
    drop(part_f);
    tokio::fs::write(
        dir.path().join("out.bin.part.id"),
        format!("{}\n{}", entry.chunk_object_ids[0], total),
    )
    .await
    .unwrap();

    download_single_file(&shared, entry, &dest, no_cancel(), |_, _| {})
        .await
        .unwrap();
    assert_eq!(file_blake3(&dest).await, file_blake3(&src).await);
    // сайдкар и .part после успешной докачки удалены
    assert!(tokio::fs::metadata(&part).await.is_err());
}

#[tokio::test]
async fn cancel_keeps_part_then_resume_completes() {
    let (net, sdk) = mock_sdk().await;
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("data.bin");
    let total = 4 * MIB;
    write_pattern_file(&src, total).await;
    let seed = upload_and_share(&sdk, &chunk_specs(&src, "data.bin", total, MIB)).await;

    let shared = net.shared_sdk(seed).await.unwrap();
    let tree = fetch_shared_tree(&shared).await.unwrap();
    let dest = dir.path().join("out.bin");

    let cancel = Arc::new(AtomicBool::new(true));
    let err = download_single_file(&shared, &tree[0], &dest, cancel.clone(), |_, _| {})
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        sia_core::sia_client::SiaClientError::Cancelled
    ));
    assert!(tokio::fs::metadata(dir.path().join("out.bin.part"))
        .await
        .is_ok());

    cancel.store(false, Ordering::Relaxed);
    download_single_file(&shared, &tree[0], &dest, no_cancel(), |_, _| {})
        .await
        .unwrap();
    assert_eq!(file_blake3(&dest).await, file_blake3(&src).await);
}

#[tokio::test]
async fn sdk_retries_transient_pin_failures() {
    let (net, sdk) = mock_sdk().await;
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("data.bin");
    write_pattern_file(&src, MIB).await;

    // 500 от индексатора на первые два пиннинга — SDK обязан пережить их сам
    net.set_pin_slabs_failures(2);
    let seed = upload_and_share(&sdk, &chunk_specs(&src, "data.bin", MIB, MIB)).await;
    assert!(net.pin_slabs_calls() >= 3);

    let shared = net.shared_sdk(seed).await.unwrap();
    let tree = fetch_shared_tree(&shared).await.unwrap();
    let dest = dir.path().join("out.bin");
    download_single_file(&shared, &tree[0], &dest, no_cancel(), |_, _| {})
        .await
        .unwrap();
    assert_eq!(file_blake3(&dest).await, file_blake3(&src).await);
}

#[tokio::test]
async fn lost_sectors_are_download_error_not_silent_corruption() {
    let (net, sdk) = mock_sdk().await;
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("data.bin");
    write_pattern_file(&src, MIB).await;
    let seed = upload_and_share(&sdk, &chunk_specs(&src, "data.bin", MIB, MIB)).await;

    let shared = net.shared_sdk(seed).await.unwrap();
    let tree = fetch_shared_tree(&shared).await.unwrap();
    // Объекты и слабы в индексаторе целы, сектора на хостах потеряны
    net.clear_sectors();
    let dest = dir.path().join("out.bin");
    assert!(
        download_single_file(&shared, &tree[0], &dest, no_cancel(), |_, _| {})
            .await
            .is_err()
    );
}

#[tokio::test]
async fn slow_hosts_still_complete() {
    let net = MockNetwork::new();
    let keys = net.add_hosts(HOSTS);
    let sdk = net.sdk(AppKey::import([7u8; 32])).await.unwrap();
    net.set_slow_hosts(keys, Duration::from_millis(20));

    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("data.bin");
    let total = 4 * MIB;
    write_pattern_file(&src, total).await;
    let seed = upload_and_share(&sdk, &chunk_specs(&src, "data.bin", total, MIB)).await;

    let shared = net.shared_sdk(seed).await.unwrap();
    let tree = fetch_shared_tree(&shared).await.unwrap();
    let dest = dir.path().join("out.bin");
    download_single_file(&shared, &tree[0], &dest, no_cancel(), |_, _| {})
        .await
        .unwrap();
    assert_eq!(file_blake3(&dest).await, file_blake3(&src).await);
}

/// Реальный размер чанка приложения (256 МиБ): 600 МиБ = 3 чанка.
/// sectors мок-хостов живут в ОЗУ (~2 ГиБ с избыточностью) — только по запросу.
#[tokio::test]
#[ignore = "тяжелый: ~600 МиБ данных и ~2 ГиБ ОЗУ"]
async fn large_file_with_real_chunk_size() {
    let (net, sdk) = mock_sdk().await;
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("large.bin");
    let total = 600 * MIB;
    write_pattern_file(&src, total).await;
    ensure_capacity(&sdk, total).await.unwrap();

    let specs = chunk_specs(&src, "large.bin", total, DEFAULT_CHUNK_SIZE);
    assert_eq!(specs.len(), 3);
    let seed = upload_and_share(&sdk, &specs).await;

    let shared = net.shared_sdk(seed).await.unwrap();
    let tree = fetch_shared_tree(&shared).await.unwrap();
    assert_eq!(tree[0].chunk_object_ids.len(), 3);
    let dest = dir.path().join("out.bin");
    download_single_file(&shared, &tree[0], &dest, no_cancel(), |_, _| {})
        .await
        .unwrap();
    assert_eq!(tokio::fs::metadata(&dest).await.unwrap().len(), total);
    assert_eq!(file_blake3(&dest).await, file_blake3(&src).await);
}
