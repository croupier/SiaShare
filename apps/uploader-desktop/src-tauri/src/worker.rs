use crate::db::Database;
use sia_core::models::ChunkMetadata;
use sia_core::sia_client::{
    parse_object_id, share_session_objects, upload_packed_chunks, ChunkUploadSpec,
};
use sia_storage::{PackedUploadOptions, Sdk};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Runtime};
use tokio::sync::{watch, Semaphore};

const MAX_CHUNK_ATTEMPTS: u32 = 8;
const MAX_FINALIZE_ATTEMPTS: u32 = 5;

/// Срок жизни раздачи ОТ МОМЕНТА ФИНАЛИЗАЦИИ: ключ создаётся здесь, и долгая
/// загрузка не съедает TTL. ttl_seconds=0 → None (вечная шара, SDK: no expiry).
/// Старые строки БД (ttl_seconds=NULL) → абсолютный expires_at. `now` передаётся
/// параметром ради детерминированных тестов.
fn compute_expires_at(
    ttl_seconds: Option<i64>,
    legacy_expires_at: Option<i64>,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<Option<chrono::DateTime<chrono::Utc>>, String> {
    match ttl_seconds {
        Some(0) => Ok(None),
        Some(ttl) => chrono::Duration::try_seconds(ttl)
            .and_then(|d| now.checked_add_signed(d))
            .map(Some)
            .ok_or_else(|| "срок жизни раздачи слишком велик".to_string()),
        None => Ok(legacy_expires_at.and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))),
    }
}

/// Backoff финализации: 10·2^(n-1) секунд, потолок 300 с. Чистая функция ради
/// теста; расписание повторов хранится per-session и не блокирует другие сессии.
fn finalize_retry_delay(attempt: u32) -> Duration {
    Duration::from_secs(
        10u64
            .saturating_mul(2u64.saturating_pow(attempt.saturating_sub(1)))
            .min(300),
    )
}

#[derive(Clone, serde::Serialize)]
struct ChunkProgress {
    session_id: String,
    file_id: String,
    chunk_index: u32,
    fraction: f64,
    length_bytes: u64,
}
#[derive(Clone, serde::Serialize)]
struct SessionProgress {
    session_id: String,
    uploaded: i64,
    total: i64,
    failed: i64,
    total_bytes: i64,
    uploaded_bytes: i64,
}

/// Запускает пул воркеров + финализатор. Возвращает sender для подмены SDK:
/// повторный вход просто шлет новый Arc<Sdk>, и все воркеры подхватывают его
/// на следующей итерации (иначе они навсегда остались бы на старом аккаунте).
/// Обобщен по Runtime ради тестов с tauri::test::mock_app (MockRuntime).
pub fn start_upload_workers<R: Runtime>(
    app: AppHandle<R>,
    db: Database,
    concurrency: usize,
) -> watch::Sender<Option<Arc<Sdk>>> {
    let (sdk_tx, sdk_rx) = watch::channel::<Option<Arc<Sdk>>>(None);

    let semaphore = Arc::new(Semaphore::new(concurrency));
    // Мелкие чанки пакуются в один слаб: лимит = полезные данные слаба (40 МиБ)
    let batch_cap = PackedUploadOptions::default().optimal_data_size() as u64;
    for _ in 0..concurrency {
        let (app, db, sema, mut rx) = (app.clone(), db.clone(), semaphore.clone(), sdk_rx.clone());
        tauri::async_runtime::spawn(async move {
            loop {
                // borrow().clone() отпускает Ref в конце выражения — держать его
                // через await нельзя, это заблокировало бы sender при смене SDK
                let sdk = rx.borrow_and_update().clone();
                let Some(sdk) = sdk else {
                    let _ = rx.changed().await;
                    continue;
                };

                let permit = sema.clone().acquire_owned().await.unwrap();
                let tasks = match db.claim_next_pending_batch(batch_cap) {
                    Ok(t) if !t.is_empty() => t,
                    _ => {
                        drop(permit);
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        continue;
                    }
                };
                let (app, db, sdk) = (app.clone(), db.clone(), sdk.clone());
                tauri::async_runtime::spawn(async move {
                    let _permit = permit; // освободит слот при выходе
                    let specs: Vec<ChunkUploadSpec> = tasks
                        .iter()
                        .map(|t| ChunkUploadSpec {
                            local_path: t.local_path.clone(),
                            offset_bytes: t.offset_bytes,
                            length_bytes: t.length_bytes,
                            meta: ChunkMetadata {
                                rel_path: t.rel_path.clone(),
                                chunk_idx: t.chunk_index,
                                total_chunks: t.total_chunks,
                                total_file_size: t.total_file_size,
                                chunk_size: t.chunk_size,
                                // blake3 посчитается при чтении чанка в upload_packed_chunks
                                blake3: None,
                            },
                        })
                        .collect();

                    // Прогресс по шардам: shard_size — байты С избыточностью.
                    // Ожидаемый encoded-размер пачки = число слабов × полный слаб
                    // (неполные слабы паддятся до полного размера).
                    let opts = PackedUploadOptions::default();
                    let logical: u64 = tasks.iter().map(|t| t.length_bytes).sum();
                    let slabs =
                        ((logical as f64 / opts.optimal_data_size() as f64).ceil() as u64).max(1);
                    let expected_encoded = (slabs * opts.slab_size() as u64) as f64;
                    let sent = Arc::new(AtomicU64::new(0));
                    let last_ms = Arc::new(AtomicU64::new(0));
                    let start = Instant::now();
                    // Одна доля на всю пачку, рассылаемая по всем ее чанкам:
                    // UI суммирует доли чанков — агрегированный прогресс сходится
                    let targets: Vec<(String, String, u32, u64)> = tasks
                        .iter()
                        .map(|t| {
                            (
                                t.session_id.clone(),
                                t.file_id.clone(),
                                t.chunk_index,
                                t.length_bytes,
                            )
                        })
                        .collect();
                    let (app_cb, targets_cb) = (app.clone(), targets.clone());
                    let (sent_cb, last_cb) = (sent.clone(), last_ms.clone());
                    let on_shard = move |p: sia_storage::ShardProgress| {
                        let total = sent_cb.fetch_add(p.shard_size as u64, Ordering::Relaxed)
                            + p.shard_size as u64;
                        let now = start.elapsed().as_millis() as u64;
                        let prev = last_cb.load(Ordering::Relaxed);
                        // троттлинг 250 мс без мьютекса: колбэк — Fn, не FnMut
                        if now.saturating_sub(prev) >= 250
                            && last_cb
                                .compare_exchange(prev, now, Ordering::Relaxed, Ordering::Relaxed)
                                .is_ok()
                        {
                            let fraction = (total as f64 / expected_encoded).min(1.0);
                            for (sid, fid, cidx, len) in &targets_cb {
                                let _ = app_cb.emit(
                                    "chunk-progress",
                                    ChunkProgress {
                                        session_id: sid.clone(),
                                        file_id: fid.clone(),
                                        chunk_index: *cidx,
                                        fraction,
                                        length_bytes: *len,
                                    },
                                );
                            }
                        }
                    };

                    match upload_packed_chunks(&sdk, &specs, on_shard).await {
                        Ok(objects) => {
                            for (task, obj) in tasks.iter().zip(&objects) {
                                let id_hex = obj.id().to_string();
                                match db.mark_chunk_uploaded(
                                    &task.file_id,
                                    task.chunk_index,
                                    &id_hex,
                                ) {
                                    Ok(old_id) => {
                                        // Чистим сироту от прошлой неудачной попытки: слабы
                                        // пиннятся уже во время upload, без удаления текла бы квота
                                        if let Some(old) = old_id {
                                            if let Ok(old_id) = parse_object_id(&old) {
                                                let _ = sdk.delete_object(&old_id).await;
                                                // best-effort
                                            }
                                        }
                                    }
                                    Err(e) => eprintln!("БД: не удалось отметить чанк: {e}"),
                                }
                            }
                            let sessions: HashSet<String> =
                                tasks.iter().map(|t| t.session_id.clone()).collect();
                            for sid in sessions {
                                let _ = app.emit("chunk-uploaded", sid.clone());
                                if let Ok((total, uploaded, failed)) = db.session_chunk_counts(&sid)
                                {
                                    let (total_bytes, uploaded_bytes) =
                                        db.session_byte_counts(&sid).unwrap_or((0, 0));
                                    let _ = app.emit(
                                        "session-progress",
                                        SessionProgress {
                                            session_id: sid,
                                            uploaded,
                                            total,
                                            failed,
                                            total_bytes,
                                            uploaded_bytes,
                                        },
                                    );
                                }
                            }
                        }
                        Err(err) => {
                            // Экспоненциальный backoff с потолком; после MAX_CHUNK_ATTEMPTS — failed.
                            // Пачка ретраится целиком: объекты неразделимы до finalize.
                            let attempts = tasks.iter().map(|t| t.attempts).max().unwrap_or(1);
                            let delay = Duration::from_secs(
                                (5u64 * 2u64.saturating_pow(attempts - 1)).min(300),
                            );
                            tokio::time::sleep(delay).await;
                            for task in &tasks {
                                let failed = match db.record_chunk_error(
                                    &task.file_id,
                                    task.chunk_index,
                                    &err.to_string(),
                                    MAX_CHUNK_ATTEMPTS,
                                ) {
                                    Ok(f) => f,
                                    Err(e) => {
                                        // Чанк останется 'uploading' до перезапуска,
                                        // тогда db.init сбросит его в pending
                                        eprintln!("БД: не удалось записать ошибку чанка: {e}");
                                        false
                                    }
                                };
                                let _ = app.emit("chunk-error", serde_json::json!({
                                    "session_id": task.session_id, "chunk_index": task.chunk_index,
                                    "error": err.to_string(), "failed": failed,
                                }));
                                if failed {
                                    let _ = app.emit(
                                        "session-failed",
                                        (task.session_id.clone(), err.to_string()),
                                    );
                                }
                            }
                        }
                    }
                });
            }
        });
    }

    // Единственный финализатор: превращает завершенные сессии в SharingKey.
    // Статус 'finalizing' + сохраненный seed делают повтор безопасным.
    {
        let (app, db, mut rx) = (app.clone(), db.clone(), sdk_rx.clone());
        tauri::async_runtime::spawn(async move {
            // На сессию: (номер попытки, момент следующей попытки). Backoff больше
            // не тормозит весь цикл sleep'ом — зависшая сессия просто пропускается
            // до своего next_at, остальные финализируются без задержки (head-of-line).
            let mut retry: HashMap<String, (u32, Instant)> = HashMap::new();
            loop {
                tokio::time::sleep(Duration::from_secs(2)).await;
                let Some(sdk) = rx.borrow_and_update().clone() else {
                    continue;
                };

                // Переводим готовые сессии в finalizing (атомарно, только если все чанки uploaded)
                for sid in db.uploading_session_ids().unwrap_or_default() {
                    let _ = db.try_begin_finalize(&sid);
                }
                let Ok(tasks) = db.finalizing_sessions() else {
                    continue;
                };
                for task in tasks {
                    let sid = task.session_id.clone();
                    let now = Instant::now();
                    let entry = retry.entry(sid.clone()).or_insert((0, now));
                    if now < entry.1 {
                        continue; // backoff ещё не истёк — не блокируем другие сессии
                    }
                    entry.0 += 1;
                    let n = entry.0;
                    if n > MAX_FINALIZE_ATTEMPTS {
                        let _ = db.mark_session_failed(&sid, "финализация: превышен лимит попыток");
                        let _ = app.emit(
                            "session-failed",
                            (sid.clone(), "финализация не удалась".to_string()),
                        );
                        retry.remove(&sid);
                        continue;
                    }

                    let result = async {
                        let ids: Vec<_> = db
                            .session_object_ids(&sid)
                            .map_err(|e| e.to_string())?
                            .iter()
                            .map(|s| parse_object_id(s).map_err(|e| e.to_string()))
                            .collect::<Result<_, _>>()?;
                        // TTL отсчитывается от ФИНАЛИЗАЦИИ (см. compute_expires_at)
                        let expires = compute_expires_at(
                            task.ttl_seconds,
                            task.expires_at,
                            chrono::Utc::now(),
                        )?;
                        let existing = task
                            .seed_hex
                            .as_deref()
                            .and_then(|h| hex::decode(h).ok())
                            .and_then(|v| <[u8; 32]>::try_from(v).ok());
                        let fresh = existing.is_none();
                        let seed = share_session_objects(&sdk, &task.name, expires, &ids, existing)
                            .await
                            .map_err(|e| e.to_string())?;
                        // Seed сохраняем сразу: повтор не должен плодить новые ключи
                        if fresh {
                            db.save_session_seed(&sid, &hex::encode(seed))
                                .map_err(|e| e.to_string())?;
                        }
                        Ok::<[u8; 32], String>(seed)
                    }
                    .await;

                    match result {
                        Ok(seed) => {
                            let _ = db.mark_session_completed(&sid);
                            // Освобождаем квоту от сирот неудачных попыток (индексатор
                            // сам защищает недавно загруженные слабы от premature prune)
                            let _ = sdk.prune_slabs(None).await;
                            let _ = app.emit("session-completed", (sid.clone(), seed.to_vec()));
                            retry.remove(&sid);
                        }
                        Err(e) => {
                            eprintln!("Финализация {sid}: {e}");
                            retry.insert(sid, (n, Instant::now() + finalize_retry_delay(n)));
                        }
                    }
                }
            }
        });
    }
    sdk_tx
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};

    fn ts(secs: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(secs, 0).unwrap()
    }

    #[test]
    fn test_ttl_zero_is_eternal() {
        let now = ts(1_000_000);
        assert_eq!(compute_expires_at(Some(0), None, now).unwrap(), None);
        // ttl_seconds задан — legacy expires_at игнорируется
        assert_eq!(compute_expires_at(Some(0), Some(5), now).unwrap(), None);
    }

    #[test]
    fn test_ttl_counts_from_now() {
        let now = ts(1_000_000);
        assert_eq!(
            compute_expires_at(Some(86400), None, now).unwrap(),
            Some(ts(1_086_400))
        );
    }

    #[test]
    fn test_legacy_absolute_expires() {
        let now = ts(1_000_000);
        // ttl_seconds=NULL — старая строка, expires_at абсолютный
        assert_eq!(
            compute_expires_at(None, Some(1893456000), now).unwrap(),
            Some(ts(1893456000))
        );
        assert_eq!(compute_expires_at(None, None, now).unwrap(), None);
    }

    #[test]
    fn test_ttl_overflow_is_error() {
        assert!(compute_expires_at(Some(i64::MAX), None, ts(1_000_000)).is_err());
    }

    #[test]
    fn test_finalize_retry_backoff_caps_at_300() {
        assert_eq!(finalize_retry_delay(1), Duration::from_secs(10));
        assert_eq!(finalize_retry_delay(2), Duration::from_secs(20));
        assert_eq!(finalize_retry_delay(5), Duration::from_secs(160));
        // 10·2^5 = 320 → потолок 300; дальше не растёт и не переполняется
        assert_eq!(finalize_retry_delay(6), Duration::from_secs(300));
        assert_eq!(finalize_retry_delay(100), Duration::from_secs(300));
        // attempt=0 не паникует (saturating_sub)
        assert_eq!(finalize_retry_delay(0), Duration::from_secs(10));
    }
}
