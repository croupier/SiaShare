use sia_core::crypto::{decrypt_with_password, extract_key_from_path, parse_payload, ExtractedKey};
use sia_core::downloader::{download_as_zip, download_single_file};
use sia_core::models::SharedFileEntry;
use sia_core::sia_client::{connect_shared, fetch_shared_tree};
use sia_storage::SharedSdk; // read-only SDK получателя — НЕ Sdk
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

pub struct ReceiverState {
    pub encrypted_blob: Mutex<Option<Vec<u8>>>,
    pub sdk: Mutex<Option<Arc<SharedSdk>>>,
    pub files: Mutex<Vec<SharedFileEntry>>,
    /// Флаг отмены текущей загрузки; .part одиночного файла сохраняется для докачки
    pub cancel: Arc<AtomicBool>,
    /// Гейт "одна загрузка за раз" (UI тоже дизейблит кнопки, это страховка)
    pub download_gate: Mutex<()>,
}

#[derive(serde::Serialize)]
#[serde(tag = "status", content = "data")]
pub enum InitResponse {
    Ready(Vec<SharedFileEntry>),
    PasswordRequired,
    Error(String),
}

#[derive(Clone, serde::Serialize)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: u64,
}

/// Вызывается фронтендом сразу при открытии окна: ключ из имени S#...exe / key.sia.
#[tauri::command]
pub async fn init_receiver(state: State<'_, ReceiverState>) -> Result<InitResponse, String> {
    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    match extract_key_from_path(&exe_path) {
        Ok(ExtractedKey::Plain(raw_key)) => Ok(match load_files_with_key(&state, &raw_key).await {
            Ok(files) => InitResponse::Ready(files),
            Err(e) => InitResponse::Error(e),
        }),
        Ok(ExtractedKey::Encrypted(blob)) => {
            *state.encrypted_blob.lock().await = Some(blob);
            Ok(InitResponse::PasswordRequired)
        }
        Err(e) => Ok(InitResponse::Error(e.to_string())),
    }
}

/// Ручной ввод ключа (если файл переименован мессенджером и имя потеряно):
/// пользователь вставляет строку r_.../p1_... или содержимое key.sia.
#[tauri::command]
pub async fn submit_key_payload(
    payload: String,
    state: State<'_, ReceiverState>,
) -> Result<InitResponse, String> {
    match parse_payload(&payload) {
        Ok(ExtractedKey::Plain(raw)) => Ok(match load_files_with_key(&state, &raw).await {
            Ok(files) => InitResponse::Ready(files),
            Err(e) => InitResponse::Error(e),
        }),
        Ok(ExtractedKey::Encrypted(blob)) => {
            *state.encrypted_blob.lock().await = Some(blob);
            Ok(InitResponse::PasswordRequired)
        }
        Err(e) => Ok(InitResponse::Error(e.to_string())),
    }
}

#[tauri::command]
pub async fn unlock_with_password(
    password: String,
    state: State<'_, ReceiverState>,
) -> Result<Vec<SharedFileEntry>, String> {
    let blob = state
        .encrypted_blob
        .lock()
        .await
        .clone()
        .ok_or("Зашифрованный ключ не найден")?;
    let raw_key = decrypt_with_password(&blob, &password).map_err(|e| e.to_string())?;
    load_files_with_key(&state, &raw_key).await
}

async fn load_files_with_key(
    state: &State<'_, ReceiverState>,
    raw_key: &[u8],
) -> Result<Vec<SharedFileEntry>, String> {
    let shared = connect_shared(raw_key).await.map_err(|e| e.to_string())?;
    let files = fetch_shared_tree(&shared)
        .await
        .map_err(|e| e.to_string())?;
    *state.sdk.lock().await = Some(Arc::new(shared));
    *state.files.lock().await = files.clone();
    Ok(files)
}

#[tauri::command]
pub async fn cancel_download(state: State<'_, ReceiverState>) -> Result<(), String> {
    state.cancel.store(true, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
pub async fn download_one(
    rel_path: String,
    save_to: String,
    app: AppHandle,
    state: State<'_, ReceiverState>,
) -> Result<(), String> {
    let _guard = state
        .download_gate
        .try_lock()
        .map_err(|_| "Уже идет скачивание")?;
    state.cancel.store(false, Ordering::Relaxed);
    let sdk = state
        .sdk
        .lock()
        .await
        .clone()
        .ok_or("Клиент не инициализирован")?;
    let entry = state
        .files
        .lock()
        .await
        .iter()
        .find(|f| f.rel_path == rel_path)
        .cloned()
        .ok_or("Файл не найден")?;
    let cancel = state.cancel.clone();
    download_single_file(
        &sdk,
        &entry,
        &PathBuf::from(save_to),
        &cancel,
        |downloaded, total| {
            // троттлинг уже внутри downloader'а — здесь просто emit
            let _ = app.emit("download-progress", DownloadProgress { downloaded, total });
        },
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn download_selected_zip(
    rel_paths: Vec<String>,
    save_zip_to: String,
    app: AppHandle,
    state: State<'_, ReceiverState>,
) -> Result<(), String> {
    let _guard = state
        .download_gate
        .try_lock()
        .map_err(|_| "Уже идет скачивание")?;
    state.cancel.store(false, Ordering::Relaxed);
    let sdk = state
        .sdk
        .lock()
        .await
        .clone()
        .ok_or("Клиент не инициализирован")?;
    let selected: Vec<SharedFileEntry> = state
        .files
        .lock()
        .await
        .iter()
        .filter(|f| rel_paths.contains(&f.rel_path))
        .cloned()
        .collect();
    if selected.is_empty() {
        return Err("Не выбрано ни одного файла".into());
    }
    let cancel = state.cancel.clone();
    download_as_zip(
        sdk,
        selected,
        PathBuf::from(save_zip_to),
        cancel,
        move |downloaded, total| {
            let _ = app.emit("download-progress", DownloadProgress { downloaded, total });
        },
    )
    .await
    .map_err(|e| e.to_string())
}
