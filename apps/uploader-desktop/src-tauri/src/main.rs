#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod db;
mod worker;

use commands::{
    check_saved_auth, connect_sia, generate_share_zip, get_session_progress, revoke_session,
    start_upload_session, UploaderState,
};
use db::Database;
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        // single-instance ОБЯЗАН быть первым: иначе второй экземпляр при старте
        // сбросит uploading-чанки первого (см. db.init)
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_dir)?;
            let db = Database::init(&app_data_dir.join("queue.db")).expect("SQLite");
            // Воркеры стартуют один раз и живут всю жизнь приложения;
            // подключение они получают через watch-канал (изначально None)
            let sdk_tx = worker::start_upload_workers(app.handle().clone(), db.clone(), 3);
            app.manage(UploaderState { db, sdk_tx });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            check_saved_auth,
            connect_sia,
            start_upload_session,
            generate_share_zip,
            revoke_session,
            get_session_progress
        ])
        .run(tauri::generate_context!())
        .expect("Ошибка запуска uploader-desktop");
}
