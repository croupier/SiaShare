//! Проверка относительных путей на совместимость с Windows ДО загрузки.
//! Получатель на Windows не сможет сохранить файл с запрещённым именем,
//! а коллизия регистра (1.txt / 1.TXT) привела бы к молчаливой перезаписи.
//! Решение владельца: предупреждать обо всём, выбор — игнор или отмена;
//! получателя не меняем (его sanitize_rel_path остаётся последней обороной).

use serde::Serialize;
use std::collections::HashMap;

/// Мягкий лимит полного rel_path. Жёсткий лимит Windows — 260 UTF-16 единиц
/// на полный путь; папку сохранения получателя при загрузке не знаем, поэтому
/// оставляем запас под неё и суффикс ".part". Одиночное скачивание через Rust
/// std работает и с длинными путями (\\?\), но распаковщики ZIP (Проводник)
/// всё равно упираются в 260 — предупреждаем, не блокируем.
pub const PATH_LEN_WARN: usize = 200;
/// Жёсткий лимит NTFS на один компонент пути — не обходится даже через \\?\.
pub const COMPONENT_LEN_MAX: usize = 255;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PathWarningKind {
    InvalidChars,
    ReservedName,
    TrailingDotSpace,
    ComponentTooLong,
    PathTooLong,
    CaseCollision,
}

#[derive(Debug, Clone, Serialize)]
pub struct PathWarning {
    pub rel_path: String,
    pub kind: PathWarningKind,
    pub message: String,
}

/// Длина в UTF-16 кодовых единицах — именно так Windows считает лимиты путей
/// (эмодзи и прочие surrogate pairs занимают по 2 единицы).
fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

/// Зарезервированные имена устройств Windows: нельзя использовать как имя
/// файла даже с расширением (aux.txt, CON.tar.gz). Сравниваем базу ДО первой
/// точки без учёта регистра. CONIN$/CONOUT$ — тоже зарезервированы.
fn is_reserved_name(component: &str) -> bool {
    let base = component.split('.').next().unwrap_or(component);
    let base = base.to_ascii_uppercase();
    matches!(
        base.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || (base.len() == 4
        && (base.starts_with("COM") || base.starts_with("LPT"))
        && base.as_bytes()[3].is_ascii_digit()
        && base.as_bytes()[3] != b'0')
}

fn forbidden_chars(component: &str) -> Vec<char> {
    component
        .chars()
        .filter(|c| matches!(c, '<' | '>' | ':' | '"' | '\\' | '|' | '?' | '*') || c.is_control())
        .collect()
}

/// Чистая функция: список rel_path (прямые слеши, как после обхода папок) ->
/// список предупреждений. Порядок предупреждений детерминирован: по файлам,
/// внутри файла — по компонентам, коллизия регистра — в конце прохода.
pub fn validate_windows_paths(rel_paths: &[String]) -> Vec<PathWarning> {
    let mut warnings = Vec::new();
    let mut seen_lower: HashMap<String, String> = HashMap::new();

    for rel in rel_paths {
        if utf16_len(rel) > PATH_LEN_WARN {
            warnings.push(PathWarning {
                rel_path: rel.clone(),
                kind: PathWarningKind::PathTooLong,
                message: format!(
                    "путь длиннее {PATH_LEN_WARN} символов — у получателя может не хватить лимита Windows (260 на полный путь)"
                ),
            });
        }

        for comp in rel.split('/') {
            let bad = forbidden_chars(comp);
            if !bad.is_empty() {
                warnings.push(PathWarning {
                    rel_path: rel.clone(),
                    kind: PathWarningKind::InvalidChars,
                    message: format!(
                        "компонент «{comp}» содержит запрещённые символы: {bad:?} (< > : \" \\ | ? * и управляющие недопустимы)"
                    ),
                });
            }
            if is_reserved_name(comp) {
                warnings.push(PathWarning {
                    rel_path: rel.clone(),
                    kind: PathWarningKind::ReservedName,
                    message: format!(
                        "«{comp}» — зарезервированное имя Windows (CON, PRN, AUX, NUL, COM1–9, LPT1–9), даже с расширением"
                    ),
                });
            }
            if comp.ends_with('.') || comp.ends_with(' ') {
                warnings.push(PathWarning {
                    rel_path: rel.clone(),
                    kind: PathWarningKind::TrailingDotSpace,
                    message: format!(
                        "компонент «{comp}» оканчивается точкой или пробелом — Windows их молча отбрасывает"
                    ),
                });
            }
            if utf16_len(comp) > COMPONENT_LEN_MAX {
                warnings.push(PathWarning {
                    rel_path: rel.clone(),
                    kind: PathWarningKind::ComponentTooLong,
                    message: format!(
                        "компонент длиннее {COMPONENT_LEN_MAX} символов — жёсткий лимит NTFS, файл не создать"
                    ),
                });
            }
        }

        // Windows сравнивает пути без учёта регистра: 1.txt и 1.TXT — один
        // файл, второй молча перезапишет первого. to_lowercase() — Unicode-
        //aware, близко к ordinal-ignore-case Windows для практичных случаев.
        let lower = rel.to_lowercase();
        match seen_lower.get(&lower) {
            Some(first) if first != rel => {
                warnings.push(PathWarning {
                    rel_path: rel.clone(),
                    kind: PathWarningKind::CaseCollision,
                    message: format!(
                        "конфликт регистра с «{first}» — на Windows это один и тот же файл, второй молча перезапишет первый"
                    ),
                });
            }
            None => {
                seen_lower.insert(lower, rel.clone());
            }
            _ => {} // точный дубликат — его переименует dedupe_rel_paths
        }
    }
    warnings
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(paths: &[&str]) -> Vec<PathWarningKind> {
        let owned: Vec<String> = paths.iter().map(|s| s.to_string()).collect();
        validate_windows_paths(&owned)
            .iter()
            .map(|w| w.kind)
            .collect()
    }

    #[test]
    fn test_clean_paths_no_warnings() {
        assert!(kinds(&["Docs/2026/report.pdf", "movie.mkv", "dir/file (2).txt"]).is_empty());
    }

    #[test]
    fn test_forbidden_chars() {
        for p in [
            "a<b.txt",
            "a>b.txt",
            "a:b.txt",
            "a\"b.txt",
            "a|b.txt",
            "a?b.txt",
            "a*b.txt",
            "a\\b.txt",
            "a\u{1}b.txt",
        ] {
            assert!(
                kinds(&[p]).contains(&PathWarningKind::InvalidChars),
                "не поймано: {p}"
            );
        }
    }

    #[test]
    fn test_reserved_names() {
        for p in [
            "aux.txt",
            "CON",
            "dir/com1.log",
            "nul.tar.gz",
            "CONIN$",
            "LPT9",
            "prn.md",
        ] {
            assert!(
                kinds(&[p]).contains(&PathWarningKind::ReservedName),
                "не поймано: {p}"
            );
        }
        // не зарезервированные: база имени отличается
        assert!(kinds(&[
            "console.txt",
            "com10.txt",
            "auxiliary.md",
            "com0.txt",
            "lpt0"
        ])
        .is_empty());
    }

    #[test]
    fn test_trailing_dot_space() {
        for p in ["file.", "file ", "dir./x.txt", "dir /x.txt"] {
            assert!(
                kinds(&[p]).contains(&PathWarningKind::TrailingDotSpace),
                "не поймано: {p}"
            );
        }
        // пробел/точка в СЕРЕДИНЕ имени допустимы
        assert!(kinds(&["file .txt", "my file.txt"]).is_empty());
    }

    #[test]
    fn test_component_length_utf16() {
        let ok = "a".repeat(255);
        let bad = "a".repeat(256);
        // 255 символов — компонент допустим (длинный путь даёт лишь мягкое PathTooLong)
        assert!(!kinds(&[&ok]).contains(&PathWarningKind::ComponentTooLong));
        assert!(kinds(&[&bad]).contains(&PathWarningKind::ComponentTooLong));
        // 128 эмодзи = 256 UTF-16 единиц > 255, хотя chars() == 128
        let emoji = "😀".repeat(128);
        assert!(kinds(&[&emoji]).contains(&PathWarningKind::ComponentTooLong));
    }

    #[test]
    fn test_path_length_soft_limit() {
        let ok = format!("dir/{}", "a".repeat(196)); // ровно 200
        let bad = format!("dir/{}", "a".repeat(197)); // 201
        assert_eq!(utf16_len(&ok), 200);
        assert!(kinds(&[&ok]).is_empty());
        assert!(kinds(&[&bad]).contains(&PathWarningKind::PathTooLong));
    }

    #[test]
    fn test_case_collision() {
        let w = kinds(&["1.txt", "1.TXT"]);
        assert_eq!(w, vec![PathWarningKind::CaseCollision]);
        assert!(kinds(&["dir/a", "DIR/a"]).contains(&PathWarningKind::CaseCollision));
        // точный дубликат — не коллизия (его переименует dedupe_rel_paths)
        assert!(kinds(&["a.txt", "a.txt"]).is_empty());
    }

    #[test]
    fn test_multiple_issues_one_file() {
        // aux. — и зарезервированное имя, и trailing dot
        let w = kinds(&["aux."]);
        assert!(w.contains(&PathWarningKind::ReservedName));
        assert!(w.contains(&PathWarningKind::TrailingDotSpace));
    }
}
