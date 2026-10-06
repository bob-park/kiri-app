mod commands;
mod settings;
mod windows;

use settings::SettingsState;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            app.manage(SettingsState::new(
                app.path().app_config_dir()?.join("settings.json"),
            ));
            windows::show_main(app.handle()).map_err(std::io::Error::other)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::patch_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running kiri");
}
