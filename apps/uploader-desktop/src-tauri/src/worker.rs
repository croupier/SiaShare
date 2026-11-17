use crate::db::Database;
use sia_core::models::ChunkMetadata;
use sia_core::sia_client::{parse_object_id, share_session_objects, upload_chunk};
use sia_storage::{Sdk, UploadOptions};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tokio::sync::{watch, Semaphore};

const MAX_CHUNK_ATTEMPTS: u32 = 8;
const MAX_FINALIZE_ATTEMPTS: u32 = 5;

#[derive(Clone, serde::Serialize)]
struct ChunkProgress { session_id: String, file_id: String, chunk_index: u32, fraction: f64 }
#[derive(Clone, serde::Serialize)]
struct SessionProgress { session_id: String, uploaded: i64, total: i64, failed: i64 }

/// Запускает пул воркеров + финализатор. Возвращает sender для подмены SDK:
/// повторный вход просто шлет новый Arc<Sdk>, и все воркеры подхватывают его
/// на следующей итерации (иначе они навсегда остались бы на старом аккаунте).
pub fn start_upload_workers(app: AppHandle, db: Database, concurrency: usize) -> watch::Sender<Option<Arc<Sdk>>> {
    let (sdk_tx, sdk_rx) = watch::channel::<Option<Arc<Sdk>>>(None);

    let semaphore = Arc::new(Semaphore::new(concurrency));
    for _ in 0..concurrency {
        let (app, db, sema, mut rx) = (app.clone(), db.clone(), semaphore.clone(), sdk_rx.clone());
        tauri::async_runtime::spawn(async move {
            loop {
                // borrow().clone() отпускает Ref в конце выражения — держать его
                // через await нельзя, это заблокировало бы sender при смене SDK
                let sdk = rx.borrow_and_update().clone();
                let Some(sdk) = sdk else { let _ = rx.changed().await; continue };

                let permit = sema.clone().acquire_owned().await.unwrap();
                let task = match db.claim_next_pending_chunk() {
                    Ok(Some(t)) => t,
                    _ => { drop(permit); tokio::time::sleep(Duration::from_secs(1)).await; continue }
                };
                let (app, db, sdk) = (app.clone(), db.clone(), sdk.clone());
                tauri::async_runtime::spawn(async move {
                    let _permit = permit; // освободит слот при выходе
                    let meta = ChunkMetadata {
                        rel_path: task.rel_path.clone(),
                        chunk_idx: task.chunk_index,
                        total_chunks: task.total_chunks,
                        total_file_size: task.total_file_size,
                        chunk_size: task.chunk_size,
                    };

                    // Прогресс по шардам: shard_size — байты С избыточностью,
                    // поэтому делим на ожидаемый encoded-размер чанка (~3x при 10+20)
                    let opts = UploadOptions::default();
                    let ratio = opts.slab_size() as f64 / opts.optimal_data_size() as f64;
                    let expected_encoded = (task.length_bytes as f64 * ratio).max(1.0);
                    let sent = Arc::new(AtomicU64::new(0));
                    let last_ms = Arc::new(AtomicU64::new(0));
                    let start = Instant::now();
                    let (app_cb, sid, fid, cidx) = (app.clone(), task.session_id.clone(), task.file_id.clone(), task.chunk_index);
                    let (sent_cb, last_cb) = (sent.clone(), last_ms.clone());
                    let on_shard = move |p: sia_storage::ShardProgress| {
                        let total = sent_cb.fetch_add(p.shard_size as u64, Ordering::Relaxed) + p.shard_size as u64;
                        let now = start.elapsed().as_millis() as u64;
                        let prev = last_cb.load(Ordering::Relaxed);
                        // троттлинг 250 мс без мьютекса: колбэк — Fn, не FnMut
                        if now.saturating_sub(prev) >= 250
                            && last_cb.compare_exchange(prev, now, Ordering::Relaxed, Ordering::Relaxed).is_ok()
                        {
                            let _ = app_cb.emit("chunk-progress", ChunkProgress {
                                session_id: sid.clone(), file_id: fid.clone(), chunk_index: cidx,
                                fraction: (total as f64 / expected_encoded).min(1.0),
                            });
                        }
                    };

                    match upload_chunk(&sdk, Path::new(&task.local_path), task.offset_bytes, task.length_bytes, &meta, on_shard).await {
                        Ok(obj) => {
                            let id_hex = obj.id().to_string();
                            match db.mark_chunk_uploaded(&task.file_id, task.chunk_index, &id_hex) {
                                Ok(old_id) => {
                                    // Чистим сироту от прошлой неудачной попытки: слабы
                                    // пиннятся уже во время upload, без удаления текла бы квота
                                    if let Some(old) = old_id {
                                        if let Ok(old_id) = parse_object_id(&old) {
                                            let _ = sdk.delete_object(&old_id).await; // best-effort
                                        }
                                    }
                                    let _ = app.emit("chunk-uploaded", task.session_id.clone());
                                    if let Ok((total, uploaded, failed)) = db.session_chunk_counts(&task.session_id) {
                                        let _ = app.emit("session-progress", SessionProgress {
                                            session_id: task.session_id.clone(), uploaded, total, failed,
                                        });
                                    }
                                }
                                Err(e) => eprintln!("БД: не удалось отметить чанк: {e}"),
                            }
                        }
                        Err(err) => {
                            // Экспоненциальный backoff с потолком; после MAX_CHUNK_ATTEMPTS — failed
                            let delay = Duration::from_secs((5u64 * 2u64.saturating_pow(task.attempts - 1)).min(300));
                            tokio::time::sleep(delay).await;
                            let failed = db.record_chunk_error(&task.file_id, task.chunk_index, &err.to_string(), MAX_CHUNK_ATTEMPTS).unwrap_or(false);
                            let _ = app.emit("chunk-error", serde_json::json!({
                                "session_id": task.session_id, "chunk_index": task.chunk_index,
                                "error": err.to_string(), "failed": failed,
                            }));
                            if failed {
                                let _ = app.emit("session-failed", (task.session_id.clone(), err.to_string()));
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
            let mut attempts: HashMap<String, u32> = HashMap::new();
            loop {
                tokio::time::sleep(Duration::from_secs(2)).await;
                let Some(sdk) = rx.borrow_and_update().clone() else { continue };

                // Переводим готовые сессии в finalizing (атомарно, только если все чанки uploaded)
                for sid in db.uploading_session_ids().unwrap_or_default() {
                    let _ = db.try_begin_finalize(&sid);
                }
                let Ok(Some(task)) = db.next_finalizing_session() else { continue };

                let n = attempts.entry(task.session_id.clone()).or_insert(0);
                *n += 1;
                if *n > MAX_FINALIZE_ATTEMPTS {
                    let _ = db.mark_session_failed(&task.session_id, "финализация: превышен лимит попыток");
                    let _ = app.emit("session-failed", (task.session_id.clone(), "финализация не удалась".to_string()));
                    continue;
                }

                let result = async {
                    let ids: Vec<_> = db.session_object_ids(&task.session_id)
                        .map_err(|e| e.to_string())?
                        .iter().map(|s| parse_object_id(s).map_err(|e| e.to_string())).collect::<Result<_, _>>()?;
                    // TTL — из БД (выбор пользователя), а не хардкод 7 дней
                    let expires = task.expires_at.and_then(|ts| chrono::DateTime::from_timestamp(ts, 0));
                    let existing = task.seed_hex.as_deref()
                        .and_then(|h| hex::decode(h).ok())
                        .and_then(|v| <[u8; 32]>::try_from(v).ok());
                    let fresh = existing.is_none();
                    let seed = share_session_objects(&sdk, &task.name, expires, &ids, existing)
                        .await.map_err(|e| e.to_string())?;
                    // Seed сохраняем сразу: повтор не должен плодить новые ключи
                    if fresh {
                        db.save_session_seed(&task.session_id, &hex::encode(seed)).map_err(|e| e.to_string())?;
                    }
                    Ok::<[u8; 32], String>(seed)
                }.await;

                match result {
                    Ok(seed) => {
                        let _ = db.mark_session_completed(&task.session_id);
                        // Освобождаем квоту от сирот неудачных попыток (индексатор
                        // сам защищает недавно загруженные слабы от premature prune)
                        let _ = sdk.prune_slabs(None).await;
                        let _ = app.emit("session-completed", (task.session_id.clone(), seed.to_vec()));
                        attempts.remove(&task.session_id);
                    }
                    Err(e) => {
                        eprintln!("Финализация {}: {e}", task.session_id);
                        tokio::time::sleep(Duration::from_secs((10 * 2u64.saturating_pow(*n - 1)).min(300))).await;
                    }
                }
            }
        });
    }
    sdk_tx
}
