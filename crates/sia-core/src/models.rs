use serde::{Deserialize, Serialize};

/// Метаданные чанка, записываемые в Sia Indexer (шифруются SDK вместе с объектом).
/// Позволяют восстановить структуру папок и порядок склейки без внешних серверов.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChunkMetadata {
    /// Относительный путь внутри раздачи с прямыми слешами (например, "Docs/2026/Report.pdf")
    pub rel_path: String,
    /// Индекс чанка (0, 1, 2...)
    pub chunk_idx: u32,
    /// Общее число чанков файла
    pub total_chunks: u32,
    /// Полный размер исходного файла в байтах
    pub total_file_size: u64,
    /// Размер одного чанка (по умолчанию 256 МиБ)
    pub chunk_size: u64,
    /// blake3 содержимого чанка (hex, 32 байта). Заполняется отправителем при
    /// чтении чанка; получатель сверяет после скачивания. #[serde(default)]:
    /// старые раздачи без поля → None (проверка пропускается, остаются Merkle-пруфы).
    #[serde(default)]
    pub blake3: Option<String>,
}

/// Агрегированное описание файла для отображения в UI получателя
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SharedFileEntry {
    pub rel_path: String,
    pub total_size: u64,
    pub total_chunks: u32,
    /// Размер чанка из метаданных отправителя. НЕ константа: докачка и проверка
    /// размеров обязаны использовать именно это значение, иначе смена размера
    /// чанка в новой версии отправителя сломает получателя.
    pub chunk_size: u64,
    /// Hex-идентификаторы объектов Sia (Hash256), строго отсортированные по chunk_idx
    pub chunk_object_ids: Vec<String>,
    /// blake3 каждого чанка (hex) в том же порядке, что chunk_object_ids.
    /// None у чанка = legacy-раздача без хэша (проверка пропускается).
    /// #[serde(default)]: старый IPC без поля → пустой vec.
    #[serde(default)]
    pub chunk_blake3: Vec<Option<String>>,
}

/// Защита от zip-slip/выхода за каталог: rel_path приходит из ЧУЖИХ метаданных
/// и не является доверенным. Разрешаем только относительные пути с прямыми
/// слешами, без "..", ".", пустых сегментов, дисков и обратных слешей.
/// None = путь небезопасен, вызывающий обязан вернуть ошибку, а не молчать.
pub fn sanitize_rel_path(path: &str) -> Option<String> {
    if path.is_empty() || path.len() > 1024 {
        return None;
    }
    // ':' блокирует буквы дисков и альтернативные потоки NTFS; '\' уже должен быть нормализован
    if path.contains('\\') || path.contains(':') || path.starts_with('/') {
        return None;
    }
    if path
        .split('/')
        .any(|seg| seg.is_empty() || seg == "." || seg == "..")
    {
        return None;
    }
    Some(path.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_rel_path() {
        assert!(sanitize_rel_path("Docs/2026/report.pdf").is_some());
        assert!(sanitize_rel_path("../evil.exe").is_none());
        assert!(sanitize_rel_path("a/../../b").is_none());
        assert!(sanitize_rel_path("/etc/passwd").is_none());
        assert!(sanitize_rel_path("C:\\Windows\\evil").is_none());
        assert!(sanitize_rel_path("C:/evil").is_none());
        assert!(sanitize_rel_path("a//b").is_none());
        assert!(sanitize_rel_path("").is_none());
    }

    #[test]
    fn test_sanitize_rel_path_length_and_dot_segment() {
        // >1024 байт — отказ (защита от раздутых метаданных)
        let long = format!("dir/{}", "a".repeat(1024));
        assert!(sanitize_rel_path(&long).is_none());
        // ровно 1024 — допустимо
        assert!(sanitize_rel_path(&"a".repeat(1024)).is_some());
        // сегмент "." запрещён
        assert!(sanitize_rel_path("a/./b").is_none());
        assert!(sanitize_rel_path(".").is_none());
    }

    #[test]
    fn test_chunk_metadata_blake3_legacy_default() {
        // Старый JSON без поля blake3 → None (обратная совместимость)
        let legacy = r#"{"rel_path":"a.bin","chunk_idx":0,"total_chunks":1,"total_file_size":10,"chunk_size":256}"#;
        let m: ChunkMetadata = serde_json::from_str(legacy).unwrap();
        assert_eq!(m.blake3, None);
        // Новое поле сериализуется и читается обратно
        let with_hash = ChunkMetadata {
            blake3: Some("ab".repeat(32)),
            ..m
        };
        let json = serde_json::to_string(&with_hash).unwrap();
        let back: ChunkMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(back, with_hash);
    }

    #[test]
    fn test_shared_file_entry_blake3_legacy_default() {
        // Старый IPC без chunk_blake3 → пустой vec
        let legacy = r#"{"rel_path":"a.bin","total_size":10,"total_chunks":1,"chunk_size":256,"chunk_object_ids":["x"]}"#;
        let e: SharedFileEntry = serde_json::from_str(legacy).unwrap();
        assert!(e.chunk_blake3.is_empty());
    }
}
