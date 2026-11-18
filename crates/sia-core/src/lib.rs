//! Общее ядро SiaShare: работа с сетью Sia, криптография ключа доступа,
//! нарезка/склейка чанков и потоковая архивация.
//! Крейт не зависит от Tauri и тестируется обычным `cargo test -p sia-core`.

pub mod crypto;
pub mod downloader;
pub mod models;
pub mod sia_client;
pub mod validate;
