fn main() {
    // Tauri тянет comctl32 v6 (TaskDialogIndirect): без манифеста Common-Controls
    // тестовый exe падает при загрузке с STATUS_ENTRYPOINT_NOT_FOUND. В exe
    // приложений манифест встраивает tauri-build, здесь повторяем его для тестов.
    #[cfg(windows)]
    tauri_winres::WindowsResource::new()
        .set_manifest(include_str!("windows-app-manifest.xml"))
        .compile_for_everything()
        .expect("встраивание манифеста Common-Controls v6");
}
