#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod db;
mod worker;

use commands::{
    check_saved_auth, cleanup_session, connect_sia, generate_share_zip, get_session_progress,
    list_sessions, revoke_session, start_upload_session, UploaderState,
};
use db::Database;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// Очередь живет в storage/ в корне проекта (видна и редактируема при разработке),
/// корень ищется вверх от пути exe по маркерам workspace. У установленной сборки
/// маркеров нет — тогда каталог данных приложения.
fn queue_db_path(app: &AppHandle) -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(root) = project_root(&exe) {
            return root.join("storage").join("queue.db");
        }
    }
    app.path()
        .app_data_dir()
        .expect("каталог данных приложения")
        .join("queue.db")
}

fn project_root(exe: &std::path::Path) -> Option<&std::path::Path> {
    exe.ancestors()
        .find(|d| d.join("Cargo.toml").is_file() && d.join("crates").is_dir())
}

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
            let db_path = queue_db_path(app.handle());
            if let Some(parent) = db_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let db = Database::init(&db_path).expect("SQLite");
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
            cleanup_session,
            get_session_progress,
            list_sessions
        ])
        .run(tauri::generate_context!())
        .expect("Ошибка запуска uploader-desktop");
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_project_root_detection() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "").unwrap();
        std::fs::create_dir(dir.path().join("crates")).unwrap();
        let exe = dir.path().join("target").join("debug").join("app.exe");
        assert_eq!(super::project_root(&exe), Some(dir.path()));
        // установленная сборка вне workspace — корня нет
        let outside = tempfile::tempdir().unwrap();
        let exe2 = outside.path().join("app.exe");
        assert_eq!(super::project_root(&exe2), None);
    }
}
