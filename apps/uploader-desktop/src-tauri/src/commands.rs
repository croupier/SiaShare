use crate::db::Database;
use sia_core::crypto::build_receiver_filename;
use sia_core::sia_client::{
    begin_connection, connect_with_app_key, ensure_capacity, finish_connection, parse_object_id,
    revoke_share,
};
use sia_core::validate::{validate_windows_paths, PathWarning};
use sia_storage::Sdk;
use std::collections::HashSet;
use std::fs::{self, File};
use std::path::Path;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;
use tokio::sync::watch;
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

const KEYRING_SERVICE: &str = "siashare-desktop";
const KEYRING_USER: &str = "sia-app-key";

pub struct UploaderState {
    pub db: Database,
    /// ЕДИНЫЙ источник правды о подключении: и команды, и воркеры читают
    /// этот watch-канал. Отдельного Mutex<Option<Arc<Sdk>>> нет — двойное
    /// состояние давало гонку при повторном входе.
    pub sdk_tx: watch::Sender<Option<Arc<Sdk>>>,
}

/// Клонирует текущий SDK из канала без блокировок: subscribe() отдает
/// receiver с актуальным значением, borrow().clone() — сам Option<Arc<Sdk>>.
fn current_sdk(state: &State<'_, UploaderState>) -> Result<Arc<Sdk>, String> {
    state
        .sdk_tx
        .subscribe()
        .borrow()
        .clone()
        .ok_or_else(|| "Сначала выполните вход в Sia Storage".into())
}

fn keyring_entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).map_err(|e| e.to_string())
}

/// 1. Автовход по ключу из СИСТЕМНОГО keychain (не plaintext-файл!).
/// Различаем «ключа нет» (Ok(false) → экран входа) и сбои хранилища/сети
/// (Err → UI показывает ошибку и даёт повтор), иначе временный сбой keychain
/// или сети молча выдавал бы себя за отсутствие ключа.
#[tauri::command]
pub async fn check_saved_auth(state: State<'_, UploaderState>) -> Result<bool, String> {
    let hex_key = match keyring_entry()?.get_password() {
        Ok(k) => k,
        Err(keyring::Error::NoEntry) => return Ok(false), // ключа нет — покажем экран входа
        Err(e) => return Err(format!("Keychain: {e}")),
    };
    let bytes = hex::decode(&hex_key).map_err(|e| format!("Повреждённый ключ в keychain: {e}"))?;
    match connect_with_app_key(&bytes).await {
        Ok(Some(sdk)) => {
            activate_sdk(&state, sdk);
            Ok(true)
        }
        // None = ключ отозван/недействителен — нужен повторный approval
        Ok(None) => Ok(false),
        Err(e) => Err(format!("Сеть Sia: {e}")),
    }
}

/// 2. Первичный вход: approval на странице индексатора + регистрация фразы.
/// seed_phrase = None -> новый аккаунт (фраза придёт событием auth-phrase);
/// Some -> вход существующей фразой.
#[tauri::command]
pub async fn connect_sia(
    seed_phrase: Option<String>,
    app: AppHandle,
    state: State<'_, UploaderState>,
) -> Result<(), String> {
    let builder = begin_connection().await.map_err(|e| e.to_string())?;
    let url = builder.response_url().to_string();
    // Открываем системный браузер и дублируем ссылку в UI (вдруг браузер не открылся)
    let _ = app.opener().open_url(&url, None::<&str>);
    let _ = app.emit("auth-url", &url);

    // Ожидание может занять минуты — invoke висит, UI живет на событиях
    let (sdk, generated, app_key) = finish_connection(builder, seed_phrase)
        .await
        .map_err(|e| e.to_string())?;
    if let Some(phrase) = generated {
        // Показываем один раз для записи; в БД/файлы фразу НЕ пишем
        let _ = app.emit("auth-phrase", phrase);
    }
    keyring_entry()?
        .set_password(&hex::encode(app_key))
        .map_err(|e| format!("Keychain: {e}"))?;
    activate_sdk(&state, sdk);
    Ok(())
}

/// Единственная точка обновления подключения: воркеры подхватят новый SDK
/// на следующей итерации своего цикла.
fn activate_sdk(state: &State<'_, UploaderState>, sdk: Sdk) {
    let _ = state.sdk_tx.send(Some(Arc::new(sdk)));
}

/// 3. Рекурсивный обход + дедупликация путей + проверка квоты + очередь.
#[tauri::command]
pub async fn start_upload_session(
    session_name: String,
    paths: Vec<String>,
    valid_days: i64,
    state: State<'_, UploaderState>,
) -> Result<String, String> {
    if valid_days < 0 {
        return Err("Срок жизни ссылки не может быть отрицательным".into());
    }
    // 0 = бессрочная раздача. TTL отсчитывается от успешной финализации,
    // а не от создания сессии — долгая загрузка не съедает срок жизни.
    let ttl_seconds = valid_days
        .checked_mul(86400)
        .ok_or("Срок жизни ссылки слишком велик")?;
    let sdk = current_sdk(&state)?;

    // Обход дерева — блокирующая ФС-операция, не держим async-рантайм
    let mut collected = tokio::task::spawn_blocking(move || collect_files_recursive(&paths))
        .await
        .map_err(|e| e.to_string())??;
    if collected.is_empty() {
        return Err("Не найдено ни одного файла".into());
    }
    dedupe_rel_paths(&mut collected);

    // Квота: сумма логических размеров против remaining_storage индексатора
    let planned: u64 = collected.iter().map(|(_, _, s)| *s).sum();
    ensure_capacity(&sdk, planned)
        .await
        .map_err(|e| e.to_string())?;

    state
        .db
        .create_session(&session_name, &collected, ttl_seconds)
        .map_err(|e| format!("Ошибка БД: {e}"))
}

/// Одноименные файлы/папки в одной сессии склеились бы получателем в один
/// файл — добавляем суффикс " (2)", " (3)"... перед расширением.
fn dedupe_rel_paths(files: &mut [(String, String, u64)]) {
    let mut seen: HashSet<String> = HashSet::new();
    for (_, rel, _) in files.iter_mut() {
        if seen.insert(rel.clone()) {
            continue;
        }
        let p = Path::new(rel.as_str());
        let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
        let ext = p.extension().and_then(|s| s.to_str());
        let parent = p.parent().filter(|x| !x.as_os_str().is_empty());
        for i in 2.. {
            let name = match ext {
                Some(e) => format!("{stem} ({i}).{e}"),
                None => format!("{stem} ({i})"),
            };
            let candidate = match parent {
                Some(dir) => format!("{}/{name}", dir.display()),
                None => name,
            };
            if seen.insert(candidate.clone()) {
                *rel = candidate;
                break;
            }
        }
    }
}

/// Обходит список путей (файлов или директорий) -> (local_path, rel_path, size).
pub fn collect_files_recursive(
    input_paths: &[String],
) -> Result<Vec<(String, String, u64)>, String> {
    let mut result = Vec::new();
    for raw_path in input_paths {
        let root = Path::new(raw_path);
        if !root.exists() {
            return Err(format!("Путь не найден: {raw_path}"));
        }
        if root.is_file() {
            let meta = fs::metadata(root).map_err(|e| e.to_string())?;
            let file_name = root
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("file.bin")
                .to_string();
            result.push((raw_path.clone(), file_name, meta.len()));
        } else if root.is_dir() {
            // Берем родительскую директорию, чтобы имя выбранной папки вошло в rel_path
            let base_parent = root.parent().unwrap_or(root);
            // Ошибки обхода (например, permission denied) НЕ проглатываем:
            // молчаливый пропуск подпапки выдал бы неполную раздачу как целую
            for entry in WalkDir::new(root) {
                let entry = entry.map_err(|e| e.to_string())?;
                if entry.file_type().is_file() {
                    let full_path = entry.path();
                    let meta = entry.metadata().map_err(|e| e.to_string())?;
                    // Нормализуем разделители путей в '/' для кроссплатформенности
                    let rel_path = full_path
                        .strip_prefix(base_parent)
                        .unwrap_or(full_path)
                        .to_string_lossy()
                        .replace('\\', "/");
                    result.push((
                        full_path.to_string_lossy().to_string(),
                        rel_path,
                        meta.len(),
                    ));
                }
            }
        }
    }
    Ok(result)
}

/// Подтверждение перед загрузкой: сколько файлов и их суммарный размер
/// в выбранном наборе путей (папки обходятся рекурсивно, как при загрузке).
#[tauri::command]
pub async fn describe_selection(paths: Vec<String>) -> Result<(u64, u64), String> {
    let collected = tokio::task::spawn_blocking(move || collect_files_recursive(&paths))
        .await
        .map_err(|e| e.to_string())??;
    let total_bytes: u64 = collected.iter().map(|(_, _, s)| *s).sum();
    Ok((collected.len() as u64, total_bytes))
}

/// Preflight: проверка путей на совместимость с Windows ДО постановки в очередь.
/// UI показывает список предупреждений и даёт выбор: игнор (грузим как есть)
/// или отмена (пользователь исправляет имена сам). Получателя не трогаем.
#[tauri::command]
pub async fn validate_paths(paths: Vec<String>) -> Result<Vec<PathWarning>, String> {
    let collected = tokio::task::spawn_blocking(move || collect_files_recursive(&paths))
        .await
        .map_err(|e| e.to_string())??;
    let rels: Vec<String> = collected.into_iter().map(|(_, rel, _)| rel).collect();
    Ok(validate_windows_paths(&rels))
}

/// 4. Собирает Sia.zip: шаблон receiver-micro + имя S#r_/S#p1_<ключ>.exe.
#[tauri::command]
pub async fn generate_share_zip(
    output_zip_path: String,
    sharing_key_bytes: Vec<u8>,
    password: Option<String>,
    app: AppHandle,
) -> Result<(), String> {
    // Имя шаблона зафиксировано mainBinaryName в tauri.conf.json receiver'а
    let template_name = if cfg!(target_os = "windows") {
        "resources/receiver-micro.exe"
    } else {
        "resources/receiver-micro"
    };
    let template_path = app
        .path()
        .resolve(template_name, tauri::path::BaseDirectory::Resource)
        .map_err(|e| format!("Шаблон микро-клиента не найден: {e}"))?;
    let ext = if cfg!(target_os = "windows") {
        "exe"
    } else {
        ""
    };
    // Файловые операции и Argon2id (если задан пароль) — в блокирующем потоке,
    // чтобы не тормозить async-рантайм
    tokio::task::spawn_blocking(move || {
        let inner_exe_name = build_receiver_filename(&sharing_key_bytes, password.as_deref(), ext)
            .map_err(|e| e.to_string())?;
        let mut template_file = File::open(&template_path)
            .map_err(|e| format!("Не удалось открыть шаблон {template_path:?}: {e}"))?;
        let zip_file = File::create(&output_zip_path).map_err(|e| e.to_string())?;
        let mut zip = ZipWriter::new(zip_file);
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o755); // исполняемый файл для Linux/macOS
        zip.start_file(&inner_exe_name, options)
            .map_err(|e| e.to_string())?;
        // Потоковое копирование шаблона в ZIP без загрузки всего exe в ОЗУ
        std::io::copy(&mut template_file, &mut zip).map_err(|e| e.to_string())?;
        zip.finish().map_err(|e| e.to_string())?;
        Ok::<(), String>(())
    })
    .await
    .map_err(|e| e.to_string())??;
    Ok(())
}

/// 5. Досрочный отзыв раздачи. Токены скачивания доживают до ~5 минут.
#[tauri::command]
pub async fn revoke_session(
    session_id: String,
    state: State<'_, UploaderState>,
) -> Result<(), String> {
    let sdk = current_sdk(&state)?;
    let seed_hex = state
        .db
        .session_seed(&session_id)
        .map_err(|e| e.to_string())?
        .ok_or("У сессии нет ключа")?;
    let seed: [u8; 32] = hex::decode(&seed_hex)
        .map_err(|e| e.to_string())?
        .try_into()
        .map_err(|_| "seed должен быть 32 байта")?;
    revoke_share(&sdk, seed).await.map_err(|e| e.to_string())?;
    state
        .db
        .mark_session_revoked(&session_id)
        .map_err(|e| e.to_string())
}

/// 6. Очистка квоты: удаляет объекты проваленной/отозванной раздачи и подчищает
/// незакрепленные слабы (сироты неудачных попыток не имеют id в БД).
/// Активную (completed) раздачу удалить нельзя — по ней еще качают получатели.
#[tauri::command]
pub async fn cleanup_session(
    session_id: String,
    state: State<'_, UploaderState>,
) -> Result<(), String> {
    let sdk = current_sdk(&state)?;
    let status = state
        .db
        .session_status(&session_id)
        .map_err(|e| e.to_string())?
        .ok_or("Сессия не найдена")?;
    if status != "failed" && status != "revoked" {
        return Err("Очистка доступна только для проваленных или отозванных раздач".into());
    }
    let ids = state
        .db
        .session_object_ids(&session_id)
        .map_err(|e| e.to_string())?;
    let mut errors = 0usize;
    for id in &ids {
        match parse_object_id(id) {
            Ok(h) => {
                if sdk.delete_object(&h).await.is_err() {
                    errors += 1;
                }
            }
            Err(_) => errors += 1,
        }
    }
    // Не помечаем cleaned при сетевых сбоях: часть объектов осталась бы в квоте
    if errors > 0 {
        return Err(format!(
            "Не удалось удалить {errors} из {} объектов — повторите при доступной сети",
            ids.len()
        ));
    }
    let _ = sdk.prune_slabs(None).await;
    state
        .db
        .mark_session_cleaned(&session_id)
        .map_err(|e| e.to_string())
}

/// Прогресс сессии для инициализации UI сразу после создания/перезапуска:
/// (чанков всего, загружено, провалено, байт всего, байт загружено).
#[tauri::command]
pub async fn get_session_progress(
    session_id: String,
    state: State<'_, UploaderState>,
) -> Result<(i64, i64, i64, i64, i64), String> {
    let (total, uploaded, failed) = state
        .db
        .session_chunk_counts(&session_id)
        .map_err(|e| e.to_string())?;
    let (total_bytes, uploaded_bytes) = state
        .db
        .session_byte_counts(&session_id)
        .map_err(|e| e.to_string())?;
    Ok((total, uploaded, failed, total_bytes, uploaded_bytes))
}

/// Список сессий (новые первыми) — UI восстанавливает активную раздачу после перезапуска.
#[tauri::command]
pub async fn list_sessions(
    state: State<'_, UploaderState>,
) -> Result<Vec<crate::db::SessionInfo>, String> {
    state.db.list_sessions().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_recursive_folder_scan() {
        let dir = tempdir().unwrap();
        let sub = dir.path().join("My Project").join("docs");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("readme.md"), b"hello").unwrap();
        let target = dir.path().join("My Project").to_string_lossy().to_string();
        let files = collect_files_recursive(&[target]).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].1, "My Project/docs/readme.md");
        assert_eq!(files[0].2, 5);
    }

    #[test]
    fn test_missing_path_is_error_not_silent_skip() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("no_such_dir").to_string_lossy().to_string();
        assert!(collect_files_recursive(&[missing]).is_err());
    }

    #[test]
    fn test_dedupe_rel_paths() {
        let mut files = vec![
            ("a".into(), "doc.txt".into(), 1),
            ("b".into(), "doc.txt".into(), 2),
            ("c".into(), "dir/doc.txt".into(), 3),
            ("d".into(), "doc.txt".into(), 4),
        ];
        dedupe_rel_paths(&mut files);
        let rels: Vec<_> = files.iter().map(|f| f.1.clone()).collect();
        assert_eq!(
            rels,
            vec!["doc.txt", "doc (2).txt", "dir/doc.txt", "doc (3).txt"]
        );
    }
}
