use crate::models::{sanitize_rel_path, SharedFileEntry};
use crate::sia_client::{parse_object_id, SiaClientError};
use sia_storage::{DownloadOptions, SharedSdk};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
// Синхронный Write нужен для ZipWriter<File>: AsyncWriteExt ниже по файлу
// покрывает только запись .part, а ZIP собирается синхронным крейтом `zip`.
use std::io::Write as _;
use std::time::{Duration, Instant};
use tokio::fs::{self, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, SeekFrom};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

/// Точка докачки: сколько полных чанков лежит в .part и сколько байт скачано
/// от следующего (неполного). Байты в .part проверены Merkle-пруфами SDK до
/// записи, поэтому продолжать можно с произвольного смещения, а не только
/// с границы чанка.
pub fn calculate_resume_chunk(existing_part_size: u64, chunk_size: u64) -> (usize, u64) {
    ((existing_part_size / chunk_size) as usize, existing_part_size % chunk_size)
}

/// Ожидаемая длина чанка idx: последний чанк короче, пустой файл = 0.
fn expected_chunk_len(entry: &SharedFileEntry, idx: usize) -> u64 {
    if entry.total_size == 0 {
        0
    } else {
        entry.chunk_size.min(entry.total_size - idx as u64 * entry.chunk_size)
    }
}

/// Троттлинг прогресса: событие в UI не чаще раза в 200 мс (иначе флуд
/// из десятков тысяч emit'ов на многогигабайтных файлах).
struct Throttle(Option<Instant>);
impl Throttle {
    fn new() -> Self {
        Self(None)
    }
    fn tick(&mut self, force: bool) -> bool {
        let due = self.0.map(|t| t.elapsed() >= Duration::from_millis(200)).unwrap_or(true);
        if force || due {
            self.0 = Some(Instant::now());
            true
        } else {
            false
        }
    }
}

fn dl_opts(offset: u64) -> DownloadOptions {
    DownloadOptions {
        offset,
        // ~64 МиБ буфера вместо дефолтных 10% ОЗУ / 1 ГБ
        max_buffered_chunks: Some(64),
        ..DownloadOptions::default()
    }
}

/// Скачивает один файл (1..N чанков) на диск с байтовой докачкой через .part.
/// Отмена: cancel=true -> Err(Cancelled), .part сохраняется для продолжения.
pub async fn download_single_file<F>(
    shared: &SharedSdk,
    entry: &SharedFileEntry,
    dest_path: &Path,
    cancel: &AtomicBool,
    mut on_progress: F,
) -> Result<(), SiaClientError>
where
    F: FnMut(u64, u64),
{
    // "<имя>.part" рядом с целью: простое дописывание суффикса, без
    // with_extension — не ломает имена вида archive.tar.gz / README.
    let part_path = PathBuf::from({
        let mut s = dest_path.as_os_str().to_owned();
        s.push(".part");
        s
    });

    let mut existing_len = fs::metadata(&part_path).await.map(|m| m.len()).unwrap_or(0);
    // Чужой/устаревший .part (от другого файла с тем же именем) — начинаем заново
    if existing_len > entry.total_size {
        existing_len = 0;
    }
    let (completed_chunks, partial) = calculate_resume_chunk(existing_len, entry.chunk_size);

    let mut out_file = OpenOptions::new().create(true).write(true).open(&part_path).await?;
    out_file.set_len(existing_len).await?;
    out_file.seek(SeekFrom::Start(existing_len)).await?;

    let mut downloaded = existing_len;
    let mut throttle = Throttle::new();
    on_progress(downloaded, entry.total_size);

    for (idx, object_id) in entry.chunk_object_ids.iter().enumerate() {
        if idx < completed_chunks {
            continue; // этот чанк уже целиком на диске
        }
        let expected = expected_chunk_len(entry, idx);
        // первый недоскачанный чанк продолжаем с байта partial, остальные — с нуля
        let skip = if idx == completed_chunks { partial } else { 0 };
        if expected > 0 && skip >= expected {
            continue;
        }
        let id = parse_object_id(object_id)?;
        let obj = shared.object(&id).await.map_err(|e| SiaClientError::Sdk(e.to_string()))?;
        // download — СИНХРОННЫЙ метод, возвращает AsyncRead-хэндл; drop = отмена
        let reader = shared.download(&obj, dl_opts(skip)).map_err(|e| SiaClientError::Sdk(e.to_string()))?;
        let mut reader = std::pin::pin!(reader);

        let mut chunk_read = 0u64;
        let mut buf = vec![0u8; 256 * 1024];
        loop {
            if cancel.load(Ordering::Relaxed) {
                out_file.flush().await?;
                return Err(SiaClientError::Cancelled);
            }
            let n = reader.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            out_file.write_all(&buf[..n]).await?;
            chunk_read += n as u64;
            downloaded += n as u64;
            if throttle.tick(false) {
                on_progress(downloaded, entry.total_size);
            }
        }
        // Каждый чанк обязан сойтись по размеру
        if skip + chunk_read != expected {
            return Err(SiaClientError::Integrity(format!(
                "{}: чанк {idx} скачан на {} байт вместо {}",
                entry.rel_path, skip + chunk_read, expected
            )));
        }
    }

    // Финальная проверка РАЗМЕРА: без нее недостающий чанк превратился бы
    // в "успешно скачанный" битый файл.
    if downloaded != entry.total_size {
        return Err(SiaClientError::Integrity(format!("{}: скачано {downloaded} байт из {}", entry.rel_path, entry.total_size)));
    }
    out_file.flush().await?;
    drop(out_file);
    on_progress(downloaded, entry.total_size);

    // Windows: rename поверх существующего файла падает — сначала удаляем цель
    if fs::metadata(dest_path).await.is_ok() {
        fs::remove_file(dest_path).await?;
    }
    fs::rename(&part_path, dest_path).await?;
    Ok(())
}

/// Скачивает выбранные файлы в единый ZIP потоково, без буферизации в ОЗУ.
/// Синхронный крейт `zip` (ZIP64 через large_file(true) — обязательно для
/// записей >4 ГиБ) на blocking-потоке; асинхронные чтения SDK выполняем
/// через Handle::block_on (документированный мост async -> sync).
pub async fn download_as_zip<F>(
    shared: Arc<SharedSdk>,
    entries: Vec<SharedFileEntry>,
    zip_dest_path: PathBuf,
    cancel: Arc<AtomicBool>,
    mut on_progress: F,
) -> Result<(), SiaClientError>
where
    F: FnMut(u64, u64) + Send + 'static,
{
    let handle = tokio::runtime::Handle::current();
    tokio::task::spawn_blocking(move || {
        let total_bytes: u64 = entries.iter().map(|e| e.total_size).sum();
        let mut downloaded = 0u64;
        let mut throttle = Throttle::new();

        let file = std::fs::File::create(&zip_dest_path)?;
        let mut zip = ZipWriter::new(file);

        let result = (|| -> Result<(), SiaClientError> {
            for entry in &entries {
                // rel_path из чужих метаданных — повторно санитизируем (zip-slip)
                let name = sanitize_rel_path(&entry.rel_path)
                    .ok_or_else(|| SiaClientError::Integrity(format!("небезопасный путь: {:?}", entry.rel_path)))?;
                // Stored: файлы уже сжаты/зашифрованы — не тратим CPU повторно
                let options = SimpleFileOptions::default()
                    .compression_method(CompressionMethod::Stored)
                    .large_file(true)
                    .unix_permissions(0o644);
                zip.start_file(name, options).map_err(|e| SiaClientError::Zip(e.to_string()))?;

                let mut entry_bytes = 0u64;
                handle.block_on(async {
                    for object_id in &entry.chunk_object_ids {
                        let id = parse_object_id(object_id)?;
                        let obj = shared.object(&id).await.map_err(|e| SiaClientError::Sdk(e.to_string()))?;
                        let reader = shared.download(&obj, dl_opts(0)).map_err(|e| SiaClientError::Sdk(e.to_string()))?;
                        let mut reader = std::pin::pin!(reader);
                        let mut buf = vec![0u8; 256 * 1024];
                        loop {
                            if cancel.load(Ordering::Relaxed) {
                                return Err(SiaClientError::Cancelled);
                            }
                            let n = reader.read(&mut buf).await?;
                            if n == 0 {
                                break;
                            }
                            zip.write_all(&buf[..n]).map_err(|e| SiaClientError::Zip(e.to_string()))?;
                            entry_bytes += n as u64;
                            downloaded += n as u64;
                            if throttle.tick(false) {
                                on_progress(downloaded, total_bytes);
                            }
                        }
                    }
                    Ok::<_, SiaClientError>(())
                })?;
                if entry_bytes != entry.total_size {
                    return Err(SiaClientError::Integrity(format!(
                        "{}: в ZIP попало {entry_bytes} байт из {}", entry.rel_path, entry.total_size
                    )));
                }
            }
            Ok(())
        })();

        match result {
            Ok(()) => {
                zip.finish().map_err(|e| SiaClientError::Zip(e.to_string()))?;
                on_progress(downloaded, total_bytes);
                Ok(())
            }
            Err(e) => {
                // Битый/недописанный ZIP не оставляем на диске (докачки у ZIP нет)
                drop(zip);
                let _ = std::fs::remove_file(&zip_dest_path);
                Err(e)
            }
        }
    })
    .await
    .map_err(|e| SiaClientError::Sdk(format!("ZIP-поток: {e}")))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resume_chunk_calculation() {
        assert_eq!(calculate_resume_chunk(250, 100), (2, 50));
        assert_eq!(calculate_resume_chunk(0, 100), (0, 0));
        assert_eq!(calculate_resume_chunk(200, 100), (2, 0));
    }

    #[test]
    fn test_expected_chunk_len() {
        let e = SharedFileEntry {
            rel_path: "a.bin".into(),
            total_size: 500,
            total_chunks: 2,
            chunk_size: 256,
            chunk_object_ids: vec!["x".into(), "y".into()],
        };
        assert_eq!(expected_chunk_len(&e, 0), 256);
        assert_eq!(expected_chunk_len(&e, 1), 244);
    }
}
