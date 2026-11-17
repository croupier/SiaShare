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
    if path.split('/').any(|seg| seg.is_empty() || seg == "." || seg == "..") {
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
}
