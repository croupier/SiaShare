#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use commands::{
    cancel_download, download_one, download_selected_zip, init_receiver, submit_key_payload,
    unlock_with_password, ReceiverState,
};
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
            init_receiver,
            unlock_with_password,
            submit_key_payload,
            cancel_download,
            download_one,
            download_selected_zip
        ])
        .run(tauri::generate_context!())
        .expect("Ошибка запуска receiver-micro");
}
