use crate::models::{sanitize_rel_path, ChunkMetadata, SharedFileEntry};
use chrono::{DateTime, Utc};
use futures::stream::{self, StreamExt, TryStreamExt};
use sia_storage::{
    app_id, AppKey, AppMetadata, Builder, Hash256, Object, PackedUploadOptions,
    RequestingApprovalState, Sdk, ShardProgress, SharedSdk, SharingKey, SharingKeyOptions,
};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use thiserror::Error;
use tokio::fs::File;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeekExt, ReadBuf, SeekFrom};

pub const DEFAULT_INDEXER_URL: &str = "https://sia.storage";
pub const DEFAULT_CHUNK_SIZE: u64 = 256 * 1024 * 1024; // 256 МиБ
/// Лимит одновременных share/pin-запросов к индексатору. SDK-методы берут &self
/// и не имеют разделяемого изменяемого состояния (запрос строится локально),
/// поэтому параллелить безопасно; 4 — чтобы не залить индексатор сотнями RPC.
const SHARE_PIN_CONCURRENCY: usize = 4;

/// Метаданные приложения для индексатора.
/// ВАЖНО: id участвует в деривации ключей шифрования пользователя — он
/// сгенерирован один раз и НЕ меняется после первого релиза, иначе все ранее
/// загруженные данные станут недоступны.
pub const APP_META: AppMetadata = AppMetadata {
    id: app_id!("9f3c6a1e4b7d2f8a0c5e9b3d6f1a4c7e2b5d8f0a3c6e9b1d4f7a0c3e6b9d2f5a8"),
    name: "SiaShare Desktop",
    description: "Serverless file sharing on the Sia network",
    service_url: "https://github.com/your-name/sia-share",
    logo_url: None,
    callback_url: None,
};

#[derive(Error, Debug)]
pub enum SiaClientError {
    #[error("Ошибка SDK Sia: {0}")]
    Sdk(String),
    #[error("Ошибка ввода-вывода файла: {0}")]
    Io(#[from] std::io::Error),
    #[error("Ошибка JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Ошибка ZIP: {0}")]
    Zip(String),
    /// Нарушена целостность раздачи (дыры в чанках, несовпадение размеров).
    /// Лучше ошибка, чем тихо выдать получателю битый файл.
    #[error("Целостность раздачи нарушена: {0}")]
    Integrity(String),
    #[error("Некорректный ключ или идентификатор: {0}")]
    InvalidKey(String),
    #[error("Введена сид-фраза другого аккаунта (регистрация создала бы новый пустой аккаунт)")]
    PhraseMismatch,
    #[error("Операция отменена пользователем")]
    Cancelled,
}

fn sdk_err<E: std::fmt::Display>(e: E) -> SiaClientError {
    SiaClientError::Sdk(e.to_string())
}

/// Hex-строка -> Hash256 (Display/FromStr у Hash256 — hex).
pub fn parse_object_id(s: &str) -> Result<Hash256, SiaClientError> {
    s.parse::<Hash256>()
        .map_err(|e| SiaClientError::InvalidKey(format!("id объекта '{s}': {e}")))
}

/// Возврат пользователя по сохраненному AppKey (32 байта из keyring).
/// Ok(None) = ключ больше не действителен (отозван/аккаунт удален) — нужен
/// повторный approval-флоу, а не паника.
pub async fn connect_with_app_key(saved_key: &[u8]) -> Result<Option<Sdk>, SiaClientError> {
    let seed: [u8; 32] = saved_key
        .try_into()
        .map_err(|_| SiaClientError::InvalidKey("AppKey должен быть 32 байта".into()))?;
    let key = AppKey::import(seed);
    let builder = Builder::new(DEFAULT_INDEXER_URL, APP_META).map_err(sdk_err)?;
    builder.connected(&key).await.map_err(sdk_err)
}

/// Шаг 1 первичного входа: запрос подключения. Вызывающий ОБЯЗАН показать
/// пользователю response_url() (открыть системный браузер) до wait_for_approval().
pub async fn begin_connection() -> Result<Builder<RequestingApprovalState>, SiaClientError> {
    Builder::new(DEFAULT_INDEXER_URL, APP_META)
        .map_err(sdk_err)?
        .request_connection()
        .await
        .map_err(sdk_err)
}

/// Шаг 2: дождаться одобрения на странице индексатора и зарегистрировать ключ.
/// phrase = None -> генерируем новую 12-словную фразу (возвращаем её для бэкапа).
/// phrase = Some -> вход существующего пользователя; если под этим connect-key
/// уже есть аккаунт, а фраза от ДРУГОГО — ошибка PhraseMismatch.
pub async fn finish_connection(
    builder: Builder<RequestingApprovalState>,
    phrase: Option<String>,
) -> Result<(Sdk, Option<String>, [u8; 32]), SiaClientError> {
    let approved = builder.wait_for_approval().await.map_err(sdk_err)?;
    let (phrase, generated) = match phrase {
        Some(p) => {
            if approved.reconnecting()
                && !approved
                    .matches_existing_app_key(&p)
                    .await
                    .map_err(sdk_err)?
            {
                return Err(SiaClientError::PhraseMismatch);
            }
            (p, None)
        }
        None => {
            if approved.reconnecting() {
                return Err(SiaClientError::PhraseMismatch);
            }
            let p = sia_storage::generate_recovery_phrase();
            (p.clone(), Some(p))
        }
    };
    let sdk = approved.register(&phrase).await.map_err(sdk_err)?;
    let app_key = sdk.app_key().export();
    Ok((sdk, generated, app_key))
}

/// Проверка квоты ДО постановки задачи в очередь.
/// remaining_storage — сколько логических байт еще можно запинить; на сети
/// данные занимают ~3x больше (erasure coding 10+20 по умолчанию в SDK).
pub async fn ensure_capacity(sdk: &Sdk, planned_bytes: u64) -> Result<(), SiaClientError> {
    let acc = sdk.account().await.map_err(sdk_err)?;
    if !acc.ready {
        return Err(SiaClientError::Sdk(
            "Аккаунт еще не готов (индексатор синхронизируется). Повторите через минуту.".into(),
        ));
    }
    if planned_bytes > acc.remaining_storage {
        return Err(SiaClientError::Sdk(format!(
            "Недостаточно квоты: нужно {} байт, доступно {}. Освободите место (удаление + prune).",
            planned_bytes, acc.remaining_storage
        )));
    }
    Ok(())
}

/// Срез файла для пакетной загрузки вместе с метаданными получателя.
pub struct ChunkUploadSpec {
    pub local_path: String,
    pub offset_bytes: u64,
    pub length_bytes: u64,
    pub meta: ChunkMetadata,
}

/// AsyncRead-обёртка, хэширующая байты по мере чтения (blake3). Позволяет
/// посчитать хэш чанка БЕЗ лишнего прохода по файлу и без загрузки его в ОЗУ:
/// байты идут в packed.add и параллельно в hasher. Hasher в Arc<Mutex> —
/// после finalize извлекаем готовый хэш. R: Unpin (tokio File + Take — Unpin).
struct HashingReader<R> {
    inner: R,
    hasher: Arc<Mutex<blake3::Hasher>>,
}

impl<R: AsyncRead + Unpin> AsyncRead for HashingReader<R> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let before = buf.filled().len();
        let this = &mut *self;
        let res = Pin::new(&mut this.inner).poll_read(cx, buf);
        if let Poll::Ready(Ok(())) = &res {
            let new = &buf.filled()[before..];
            if !new.is_empty() {
                this.hasher.lock().unwrap().update(new);
            }
        }
        res
    }
}

/// Загружает пачку чанков отдельными объектами, но в ОБЩИХ слабах (PackedUpload).
/// Одиночная загрузка мелкого объекта стоила бы целого слаба (30 шардов × 4 МиБ
/// ≈ 120 МиБ трафика): SDK паддит неполные слабы до полного размера.
/// Возвращает объекты в порядке `chunks`; метаданные проставляются после
/// finalize — id объекта зависит только от слабов. Каждый объект пиннится сразу.
pub async fn upload_packed_chunks<F>(
    sdk: &Sdk,
    chunks: &[ChunkUploadSpec],
    on_shard: F,
) -> Result<Vec<Object>, SiaClientError>
where
    F: Fn(ShardProgress) + Send + Sync + 'static,
{
    let opts = PackedUploadOptions {
        // По умолчанию SDK буферизует до 10% ОЗУ НА КАЖДУЮ загрузку;
        // при 3 параллельных пачках это гигабайты. 2 слаба ~ 240 МиБ на воркер.
        max_buffered_slabs: Some(2),
        shard_uploaded: Some(std::sync::Arc::new(on_shard)),
        ..PackedUploadOptions::default()
    };
    let mut packed = sdk.upload_packed(opts).map_err(sdk_err)?;
    let mut hashers: Vec<Arc<Mutex<blake3::Hasher>>> = Vec::with_capacity(chunks.len());
    for c in chunks {
        let mut file = File::open(&c.local_path).await?;
        // Файл мог усохнуть между сканом и загрузкой: take() тогда тихо прочитал
        // бы меньше байт, и получатель получил бы битый чанк. Лучше ошибка هنا.
        let needed_end = c.offset_bytes.saturating_add(c.length_bytes);
        if file.metadata().await?.len() < needed_end {
            return Err(SiaClientError::Integrity(format!(
                "{}: файл изменился (усечён) после постановки в очередь",
                c.local_path
            )));
        }
        file.seek(SeekFrom::Start(c.offset_bytes)).await?;
        // take читает строго length_bytes без загрузки чанка в ОЗУ; HashingReader
        // параллельно считает blake3 тех же байтов (лишний проход по файлу не нужен)
        let hasher = Arc::new(Mutex::new(blake3::Hasher::new()));
        let reader = HashingReader {
            inner: file.take(c.length_bytes),
            hasher: hasher.clone(),
        };
        packed.add(reader).await.map_err(sdk_err)?;
        hashers.push(hasher);
    }
    let mut objects = packed.finalize().await.map_err(sdk_err)?;
    for ((obj, c), hasher) in objects.iter_mut().zip(chunks).zip(&hashers) {
        let mut meta = c.meta.clone();
        // Хэш готов после finalize (все байты прошли через HashingReader)
        meta.blake3 = Some(hasher.lock().unwrap().finalize().to_hex().to_string());
        obj.metadata = serde_json::to_vec(&meta)?;
    }
    // Пинним параллельно (порядок не важен — все объекты должны быть закреплены).
    // objs — Copy-ссылка на срез: async move захватывает её, а не перемещает Vec,
    // иначе objects нельзя было бы вернуть. Индексы вместо iter() — ради HRTB.
    let objs = objects.as_slice();
    stream::iter(0..objs.len())
        .map(|i| async move { sdk.pin_object(&objs[i]).await.map_err(sdk_err) })
        .buffer_unordered(SHARE_PIN_CONCURRENCY)
        .try_collect::<Vec<()>>()
        .await?;
    Ok(objects)
}

/// Создает SharingKey (или переиспользует сохраненный seed при повторе) и
/// привязывает к нему объекты сессии. Идемпотентно: share_object для уже
/// привязанного объекта заменяет re-sealed ключи, поэтому упавшую финализацию
/// безопасно повторять с тем же seed.
pub async fn share_session_objects(
    sdk: &Sdk,
    description: &str,
    expires_at: Option<DateTime<Utc>>,
    object_ids: &[Hash256],
    existing_seed: Option<[u8; 32]>,
) -> Result<[u8; 32], SiaClientError> {
    let sharing_key = match existing_seed {
        Some(seed) => SharingKey::import(seed),
        None => sdk
            .create_sharing_key(SharingKeyOptions {
                description: description.to_string(),
                expires_at,
            })
            .await
            .map_err(sdk_err)?,
    };
    // Параллельная привязка: share_object идемпотентен (повтор заменяет re-sealed
    // ключи), порядок не важен. key берём ссылкой; итерация по индексам, чтобы
    // async-блок не заимствовал аргумент итератора (HRTB).
    let key = &sharing_key;
    stream::iter(0..object_ids.len())
        .map(|i| async move {
            // В БД хранятся только идентификаторы; полный Object догружаем по id.
            let obj = sdk.object(&object_ids[i]).await.map_err(sdk_err)?;
            sdk.share_object(key, &obj).await.map_err(sdk_err)
        })
        .buffer_unordered(SHARE_PIN_CONCURRENCY)
        .try_collect::<Vec<()>>()
        .await?;
    Ok(sharing_key.export())
}

/// Досрочный отзыв доступа. ВНИМАНИЕ: уже выданные токены скачивания живут
/// до ~5 минут после отзыва — это не мгновенный обрыв.
pub async fn revoke_share(sdk: &Sdk, seed: [u8; 32]) -> Result<(), SiaClientError> {
    sdk.revoke_sharing_key(&SharingKey::import(seed))
        .await
        .map_err(sdk_err)
}

/// Подключение получателя: read-only SharedSdk по 32-байтному seed.
/// Скачивание оплачивается аккаунтом владельца ключа — получателю не нужны
/// ни монеты, ни регистрация.
pub async fn connect_shared(sharing_key_bytes: &[u8]) -> Result<SharedSdk, SiaClientError> {
    let seed: [u8; 32] = sharing_key_bytes
        .try_into()
        .map_err(|_| SiaClientError::InvalidKey("SharingKey должен быть 32 байта".into()))?;
    SharedSdk::connect(DEFAULT_INDEXER_URL, seed)
        .await
        .map_err(sdk_err)
}

/// Восстанавливает дерево файлов из плоского списка объектов Sia.
/// Пагинация обязательна: иначе индексатор вернет только первую страницу.
pub async fn fetch_shared_tree(shared: &SharedSdk) -> Result<Vec<SharedFileEntry>, SiaClientError> {
    const PAGE: u64 = 100;
    let mut items: Vec<(String, Vec<u8>)> = Vec::new();
    let mut offset: u64 = 0;
    loop {
        let page = shared
            .object_summaries(Some(offset), Some(PAGE))
            .await
            .map_err(sdk_err)?;
        let got = page.len() as u64;
        items.extend(page.into_iter().map(|s| (s.id.to_string(), s.metadata)));
        if got < PAGE {
            break;
        }
        offset += got;
    }
    group_summaries_into_tree(items)
}

/// Чистая функция группировки. НЕ проглатывает ошибки: битые метаданные,
/// дыры в индексах, дубликаты и несовпадение размеров = ошибка целостности.
pub fn group_summaries_into_tree(
    items: Vec<(String, Vec<u8>)>,
) -> Result<Vec<SharedFileEntry>, SiaClientError> {
    let mut grouped: HashMap<String, Vec<(u32, String, ChunkMetadata)>> = HashMap::new();
    for (object_id, meta_bytes) in items {
        let meta: ChunkMetadata = serde_json::from_slice(&meta_bytes).map_err(|e| {
            SiaClientError::Integrity(format!("объект {object_id}: битые метаданные: {e}"))
        })?;
        let rel = sanitize_rel_path(&meta.rel_path).ok_or_else(|| {
            SiaClientError::Integrity(format!(
                "небезопасный путь в метаданных: {:?}",
                meta.rel_path
            ))
        })?;
        grouped
            .entry(rel)
            .or_default()
            .push((meta.chunk_idx, object_id, meta));
    }

    let mut files = Vec::with_capacity(grouped.len());
    for (rel_path, mut chunks) in grouped {
        // Сортируем чанки строго по возрастанию индекса (0, 1, 2...)
        chunks.sort_by_key(|(idx, _, _)| *idx);
        let first = &chunks[0].2;
        let (total, cs, n) = (first.total_file_size, first.chunk_size, first.total_chunks);
        if cs == 0 || chunks.len() != n as usize {
            return Err(SiaClientError::Integrity(format!(
                "{rel_path}: получено {} чанков из {n}",
                chunks.len()
            )));
        }
        for (i, (idx, _, m)) in chunks.iter().enumerate() {
            // непрерывность 0..n-1 ловит и дыры, и дубликаты
            if *idx != i as u32 {
                return Err(SiaClientError::Integrity(format!(
                    "{rel_path}: дыра/дубликат индексов (ожидался {i}, получен {idx})"
                )));
            }
            if m.total_file_size != total || m.chunk_size != cs || m.total_chunks != n {
                return Err(SiaClientError::Integrity(format!(
                    "{rel_path}: метаданные чанков противоречат друг другу"
                )));
            }
        }
        // Размерная сходимость: (n-1)*cs < total <= n*cs; пустой файл = 1 чанк, 0 байт.
        let dim_ok = if total == 0 {
            n == 1
        } else {
            (n as u64)
                .checked_mul(cs)
                .map(|max| (n as u64 - 1) * cs < total && total <= max)
                .unwrap_or(false)
        };
        if !dim_ok {
            return Err(SiaClientError::Integrity(format!(
                "{rel_path}: размеры не сходятся (total={total}, chunk_size={cs}, n={n})"
            )));
        }
        // ids и blake3 в одном порядке (chunks отсортирован по chunk_idx)
        let chunk_object_ids: Vec<String> = chunks.iter().map(|(_, id, _)| id.clone()).collect();
        let chunk_blake3: Vec<Option<String>> =
            chunks.iter().map(|(_, _, m)| m.blake3.clone()).collect();
        files.push(SharedFileEntry {
            rel_path,
            total_size: total,
            total_chunks: n,
            chunk_size: cs,
            chunk_object_ids,
            chunk_blake3,
        });
    }
    files.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(idx: u32, n: u32, total: u64, cs: u64) -> ChunkMetadata {
        ChunkMetadata {
            rel_path: "Folder/movie.mkv".into(),
            chunk_idx: idx,
            total_chunks: n,
            total_file_size: total,
            chunk_size: cs,
            blake3: None,
        }
    }
    fn item(id: &str, m: &ChunkMetadata) -> (String, Vec<u8>) {
        (id.into(), serde_json::to_vec(m).unwrap())
    }

    #[test]
    fn test_group_summaries_sorts_chunks_correctly() {
        let items = vec![
            item("second", &meta(1, 2, 500, 256)),
            item("first", &meta(0, 2, 500, 256)),
        ];
        let tree = group_summaries_into_tree(items).unwrap();
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].chunk_object_ids, vec!["first", "second"]);
        assert_eq!(tree[0].chunk_size, 256);
    }

    #[test]
    fn test_missing_chunk_is_error_not_silent() {
        assert!(group_summaries_into_tree(vec![item("a", &meta(0, 2, 500, 256))]).is_err());
    }

    #[test]
    fn test_duplicate_index_is_error() {
        let items = vec![
            item("a", &meta(0, 2, 500, 256)),
            item("b", &meta(0, 2, 500, 256)),
        ];
        assert!(group_summaries_into_tree(items).is_err());
    }

    #[test]
    fn test_size_mismatch_is_error() {
        let items = vec![
            item("a", &meta(0, 2, 900, 256)),
            item("b", &meta(1, 2, 900, 256)),
        ];
        assert!(group_summaries_into_tree(items).is_err());
    }

    #[test]
    fn test_unsafe_path_is_error() {
        let mut m = meta(0, 1, 10, 256);
        m.rel_path = "../evil.exe".into();
        assert!(group_summaries_into_tree(vec![item("a", &m)]).is_err());
    }

    #[test]
    fn test_empty_file_ok() {
        let tree = group_summaries_into_tree(vec![item("a", &meta(0, 1, 0, 256))]).unwrap();
        assert_eq!(tree[0].total_size, 0);
    }

    #[test]
    fn test_object_id_hex_roundtrip() {
        let id = Hash256::from([7u8; 32]);
        assert_eq!(parse_object_id(&id.to_string()).unwrap(), id);
        assert!(parse_object_id("zzz").is_err());
    }

    #[test]
    fn test_contradictory_chunk_metadata_is_error() {
        let a = meta(0, 2, 500, 256);
        let mut b = meta(1, 2, 500, 256);
        b.total_file_size = 600; // противоречит чанку 0 — ошибка, не тихая склейка
        assert!(group_summaries_into_tree(vec![item("a", &a), item("b", &b)]).is_err());
        // противоречие по chunk_size тоже ловится
        let mut c = meta(1, 2, 500, 256);
        c.chunk_size = 128;
        assert!(group_summaries_into_tree(vec![item("a", &a), item("c", &c)]).is_err());
    }

    #[test]
    fn test_tree_sorted_by_rel_path() {
        let mut z = meta(0, 1, 10, 256);
        z.rel_path = "z.txt".into();
        let mut a = meta(0, 1, 10, 256);
        a.rel_path = "a.txt".into();
        let mut m = meta(0, 1, 10, 256);
        m.rel_path = "dir/m.txt".into();
        let tree =
            group_summaries_into_tree(vec![item("z", &z), item("a", &a), item("m", &m)]).unwrap();
        let paths: Vec<_> = tree.iter().map(|f| f.rel_path.as_str()).collect();
        assert_eq!(paths, vec!["a.txt", "dir/m.txt", "z.txt"]);
    }

    #[test]
    fn test_tree_carries_blake3_in_chunk_order() {
        let mut a = meta(0, 2, 500, 256);
        a.blake3 = Some("aa".repeat(32));
        let mut b = meta(1, 2, 500, 256);
        b.blake3 = Some("bb".repeat(32));
        // перемешанный порядок входа — дерево сортирует по chunk_idx
        let tree = group_summaries_into_tree(vec![item("second", &b), item("first", &a)]).unwrap();
        assert_eq!(tree[0].chunk_object_ids, vec!["first", "second"]);
        assert_eq!(
            tree[0].chunk_blake3,
            vec![Some("aa".repeat(32)), Some("bb".repeat(32))]
        );
    }

    #[test]
    fn test_tree_legacy_blake3_is_none() {
        // старые метаданные без blake3 → None (проверка на получателе пропускается)
        let tree = group_summaries_into_tree(vec![item("a", &meta(0, 1, 10, 256))]).unwrap();
        assert_eq!(tree[0].chunk_blake3, vec![None]);
    }
}
