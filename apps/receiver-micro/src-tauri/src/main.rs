#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use receiver_micro::commands::ReceiverState;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::Mutex;

fn main() {
    tauri::Builder::default()
        // single-instance здесь НАМЕРЕННО отсутствует: два разных S#….exe —
        // это две легитимные раздачи, их можно открывать параллельно.
        .plugin(tauri_plugin_dialog::init())
        .manage(ReceiverState {
            encrypted_blob: Mutex::new(None),
            sdk: Mutex::new(None),
            files: Mutex::new(Vec::new()),
            cancel: Arc::new(AtomicBool::new(false)),
            download_gate: Mutex::new(()),
        })
        .invoke_handler(tauri::generate_handler![
            receiver_micro::commands::init_receiver,
            receiver_micro::commands::unlock_with_password,
            receiver_micro::commands::submit_key_payload,
            receiver_micro::commands::cancel_download,
            receiver_micro::commands::download_one,
            receiver_micro::commands::download_selected_zip
        ])
        .run(tauri::generate_context!())
        .expect("Ошибка запуска receiver-micro");
}
