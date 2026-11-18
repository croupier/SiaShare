use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::RngCore;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum CryptoError {
    #[error("Ключ доступа не найден ни в имени файла, ни в key.sia")]
    KeyNotFound,
    #[error("Поврежденный формат ключа")]
    InvalidFormat,
    #[error("Неверный пароль или поврежденные данные")]
    DecryptionFailed,
    #[error("Ошибка выработки ключа из пароля")]
    KeyDerivationFailed,
    #[error("Длина имени файла ({0} симв.) превышает безопасный лимит")]
    FilenameTooLong(usize),
}

/// Что удалось извлечь при старте микро-клиента
#[derive(Debug, PartialEq, Eq)]
pub enum ExtractedKey {
    /// Открытый ключ (префикс `r_`) — готов к немедленному подключению
    Plain(Vec<u8>),
    /// Зашифрованный блоб (префикс `p1_`) — нужно запросить пароль в UI
    Encrypted(Vec<u8>),
}

/// Формирует имя бинарника вида `S#r_<base64url>.exe` или `S#p1_<base64url>.exe`.
/// Base64URL без `=` — все символы допустимы в именах файлов Windows/Linux/macOS.
///
/// Версия формата зашита в префикс: `p1_` = Argon2id(19456,2,1) + AES-256-GCM.
/// Смена параметров KDF в будущем обязана идти под новым префиксом (`p2_`),
/// иначе ранее выданные файлы перестанут расшифровываться.
pub fn build_receiver_filename(
    raw_sharing_key: &[u8],
    password: Option<&str>,
    extension: &str,
) -> Result<String, CryptoError> {
    let encoded = match password {
        Some(pwd) if !pwd.is_empty() => {
            let blob = encrypt_with_password(raw_sharing_key, pwd)?;
            format!("p1_{}", URL_SAFE_NO_PAD.encode(blob))
        }
        _ => format!("r_{}", URL_SAFE_NO_PAD.encode(raw_sharing_key)),
    };
    let filename = if extension.is_empty() {
        format!("S#{encoded}")
    } else {
        format!("S#{encoded}.{extension}")
    };
    // Windows MAX_PATH = 260 символов на ВЕСЬ путь. Оставляем ~95 символов запаса
    // под папку пользователя (C:\Users\...\Downloads\...).
    if filename.len() > 165 {
        return Err(CryptoError::FilenameTooLong(filename.len()));
    }
    Ok(filename)
}

/// Извлекает ключ из имени запущенного файла (после `#`) или из соседнего key.sia.
pub fn extract_key_from_path(exe_path: &Path) -> Result<ExtractedKey, CryptoError> {
    if let Some(stem) = exe_path.file_stem().and_then(|s| s.to_str()) {
        if let Some((_, payload)) = stem.split_once('#') {
            return parse_payload(payload);
        }
    }
    // Подъем до 4 уровней: на macOS бинарник лежит в Foo.app/Contents/MacOS,
    // а key.sia пользователь кладет рядом с самим .app.
    for ancestor in exe_path.ancestors().skip(1).take(4) {
        let sidecar = ancestor.join("key.sia");
        if let Ok(content) = std::fs::read_to_string(&sidecar) {
            return parse_payload(content.trim());
        }
    }
    Err(CryptoError::KeyNotFound)
}

/// Разбор payload. Публичный — используется и ручным вводом ключа в UI.
/// Обрезаем хвост вида " (1)": браузеры и мессенджеры дописывают его к имени
/// скачанного файла, и наивный decode всего хвоста падал бы InvalidFormat.
pub fn parse_payload(payload: &str) -> Result<ExtractedKey, CryptoError> {
    let payload = payload.trim();
    let (kind, b64) = if let Some(rest) = payload.strip_prefix("r_") {
        ('r', rest)
    } else if let Some(rest) = payload
        .strip_prefix("p1_")
        .or_else(|| payload.strip_prefix("p_"))
    {
        ('p', rest) // "p_" принят для совместимости со старыми блобами
    } else {
        return Err(CryptoError::InvalidFormat);
    };
    let b64: String = b64
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    let bytes = URL_SAFE_NO_PAD
        .decode(&b64)
        .map_err(|_| CryptoError::InvalidFormat)?;
    Ok(if kind == 'r' {
        ExtractedKey::Plain(bytes)
    } else {
        ExtractedKey::Encrypted(bytes)
    })
}

/// Упаковывает данные в формат: [16 байт Salt][12 байт Nonce][Ciphertext+16 байт GCM Tag]
fn encrypt_with_password(data: &[u8], password: &str) -> Result<Vec<u8>, CryptoError> {
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    let derived_key = derive_key(password, &salt)?;
    let cipher =
        Aes256Gcm::new_from_slice(&derived_key).map_err(|_| CryptoError::KeyDerivationFailed)?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, data)
        .map_err(|_| CryptoError::KeyDerivationFailed)?;
    let mut result = Vec::with_capacity(16 + 12 + ciphertext.len());
    result.extend_from_slice(&salt);
    result.extend_from_slice(&nonce);
    result.extend_from_slice(&ciphertext);
    Ok(result)
}

/// Расшифровывает блоб из `ExtractedKey::Encrypted`.
/// Неверный пароль и поврежденные данные дают одну и ту же ошибку (GCM tag).
pub fn decrypt_with_password(
    encrypted_blob: &[u8],
    password: &str,
) -> Result<Vec<u8>, CryptoError> {
    // Минимум: 16 (salt) + 12 (nonce) + 16 (tag) = 44 байта
    if encrypted_blob.len() < 44 {
        return Err(CryptoError::InvalidFormat);
    }
    let (salt, rest) = encrypted_blob.split_at(16);
    let (nonce_bytes, ciphertext) = rest.split_at(12);
    let derived_key = derive_key(password, salt)?;
    let cipher =
        Aes256Gcm::new_from_slice(&derived_key).map_err(|_| CryptoError::DecryptionFailed)?;
    cipher
        .decrypt(Nonce::from_slice(nonce_bytes), ciphertext)
        .map_err(|_| CryptoError::DecryptionFailed)
}

/// Выработка 32-байтного ключа из пароля.
/// Параметры зафиксированы ЯВНО (OWASP-минимум Argon2id: 19 МиБ, t=2, p=1):
/// полагаться на ::default() нельзя — смена дефолтов в новой версии крейта
/// сделала бы все ранее выданные блобы нерасшифровываемыми.
fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; 32], CryptoError> {
    let params =
        Params::new(19_456, 2, 1, Some(32)).map_err(|_| CryptoError::KeyDerivationFailed)?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut output_key = [0u8; 32];
    argon
        .hash_password_into(password.as_bytes(), salt, &mut output_key)
        .map_err(|_| CryptoError::KeyDerivationFailed)?;
    Ok(output_key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;

    // Реальный SharingKey::export() в sia_storage 0.13 — ровно 32 байта.
    const KEY_LEN: usize = 32;

    #[test]
    fn test_plain_key_roundtrip() {
        let raw_key = vec![7u8; KEY_LEN];
        let filename = build_receiver_filename(&raw_key, None, "exe").unwrap();
        assert!(filename.starts_with("S#r_"));
        // "S#r_" (4) + 43 символа base64url + ".exe" (4) = 51
        assert_eq!(filename.len(), 51);
        let path = PathBuf::from(format!("C:\\Downloads\\{filename}"));
        assert_eq!(
            extract_key_from_path(&path).unwrap(),
            ExtractedKey::Plain(raw_key)
        );
    }

    #[test]
    fn test_encrypted_key_roundtrip_and_length() {
        let raw_key = vec![42u8; KEY_LEN];
        let password = "StrongPassword123!";
        let filename = build_receiver_filename(&raw_key, Some(password), "exe").unwrap();
        assert!(filename.starts_with("S#p1_"));
        assert!(
            filename.len() <= 165,
            "Имя слишком длинное: {}",
            filename.len()
        );
        let path = PathBuf::from(format!("C:\\Users\\User\\Downloads\\{filename}"));
        match extract_key_from_path(&path).unwrap() {
            ExtractedKey::Encrypted(blob) => {
                assert_eq!(
                    decrypt_with_password(&blob, "wrong_pass"),
                    Err(CryptoError::DecryptionFailed)
                );
                assert_eq!(decrypt_with_password(&blob, password).unwrap(), raw_key);
            }
            _ => panic!("Ожидался зашифрованный ключ"),
        }
    }

    #[test]
    fn test_browser_rename_suffix_is_tolerated() {
        let raw_key = vec![1u8; KEY_LEN];
        let filename = build_receiver_filename(&raw_key, None, "exe").unwrap();
        // Браузер дописал " (1)" к имени скачанного файла
        let renamed = filename.replace(".exe", " (1).exe");
        let path = PathBuf::from(renamed);
        assert_eq!(
            extract_key_from_path(&path).unwrap(),
            ExtractedKey::Plain(raw_key)
        );
    }

    #[test]
    fn test_sidecar_file_fallback() {
        let dir = tempdir().unwrap();
        let raw_key = vec![99u8; KEY_LEN];
        let payload = format!("r_{}", URL_SAFE_NO_PAD.encode(&raw_key));
        std::fs::write(dir.path().join("key.sia"), payload).unwrap();
        // Имя бинарника без '#' — ключ берется из key.sia рядом
        let exe_path = dir.path().join("receiver-micro");
        assert_eq!(
            extract_key_from_path(&exe_path).unwrap(),
            ExtractedKey::Plain(raw_key)
        );
    }

    #[test]
    fn test_sidecar_found_next_to_macos_app_bundle() {
        let dir = tempdir().unwrap();
        let raw_key = vec![5u8; KEY_LEN];
        std::fs::write(
            dir.path().join("key.sia"),
            format!("r_{}", URL_SAFE_NO_PAD.encode(&raw_key)),
        )
        .unwrap();
        let macos_dir = dir
            .path()
            .join("SiaShare.app")
            .join("Contents")
            .join("MacOS");
        std::fs::create_dir_all(&macos_dir).unwrap();
        let exe_path = macos_dir.join("receiver-micro");
        assert_eq!(
            extract_key_from_path(&exe_path).unwrap(),
            ExtractedKey::Plain(raw_key)
        );
    }

    #[test]
    fn test_filename_too_long_is_error() {
        let big_key = vec![0u8; 200]; // base64 ~268 символов > лимита 165
        assert!(matches!(
            build_receiver_filename(&big_key, None, "exe"),
            Err(CryptoError::FilenameTooLong(_))
        ));
    }

    #[test]
    fn test_filename_empty_extension_and_empty_password() {
        let key = vec![7u8; KEY_LEN];
        // Linux/macOS: расширение пустое — точки в имени нет
        let no_ext = build_receiver_filename(&key, None, "").unwrap();
        assert!(no_ext.starts_with("S#r_"));
        assert!(!no_ext.contains('.'));
        // пустой пароль эквивалентен отсутствию — обычный r_, не p1_
        let empty_pwd = build_receiver_filename(&key, Some(""), "exe").unwrap();
        assert!(empty_pwd.starts_with("S#r_"));
    }

    #[test]
    fn test_parse_payload_legacy_prefix_and_trim() {
        let b64 = URL_SAFE_NO_PAD.encode([1u8, 2, 3]);
        // legacy "p_" принят как зашифрованный блоб
        assert_eq!(
            parse_payload(&format!("p_{b64}")).unwrap(),
            ExtractedKey::Encrypted(vec![1, 2, 3])
        );
        // пробелы по краям и хвост " (1)" обрезаются
        assert_eq!(
            parse_payload(&format!("  r_{b64} (1)  ")).unwrap(),
            ExtractedKey::Plain(vec![1, 2, 3])
        );
    }

    #[test]
    fn test_parse_payload_invalid_format() {
        assert_eq!(parse_payload("xyz"), Err(CryptoError::InvalidFormat));
        assert_eq!(parse_payload(""), Err(CryptoError::InvalidFormat));
        // допустимые символы, но невалидная длина base64
        assert_eq!(parse_payload("r_A"), Err(CryptoError::InvalidFormat));
    }
}
