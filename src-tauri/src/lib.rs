mod bootstrap;
mod cli_install;
mod commands;
mod i18n;
mod ipc_server;
mod quit;
mod settings;
mod tray;
mod updater;
mod windows;

use kiri_core::{
    engine::{Engine, EnginePaths},
    model::{Job, Tools},
};
use settings::SettingsState;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};

/// externalBin 은 실행 파일 옆(Contents/MacOS, dev 에서는 target/debug)에 놓인다.
pub(crate) fn sidecar(name: &str) -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join(name)))
        .unwrap_or_else(|| PathBuf::from(name))
}

/// 엔진이 직렬로 부른다. 여기서 Engine 의 변경 메서드를 부르면 교착하므로
/// emit 과 (메인 스레드를 기다리지 않는) 트레이 갱신만 한다.
fn on_queue_change(app: &AppHandle, jobs: &[Job]) {
    let _ = app.emit("queue-changed", jobs);
    tray::relabel_queue(app, jobs);
    updater::on_queue_change(app, jobs);
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let handle = app.handle().clone();
            let path = app.path();
            app.manage(SettingsState::new(
                path.app_config_dir()?.join("settings.json"),
            ));
            app.manage(bootstrap::ToolsState::default());
            app.manage(ipc_server::IpcState::default());
            app.manage(updater::UpdateState::default());

            let bin = path.app_data_dir()?.join("bin");
            let tools = Tools {
                ytdlp: bin.join("yt-dlp"),
                deno: bin.join("deno"),
                ffmpeg: sidecar("ffmpeg"),
            };
            let paths = EnginePaths {
                queue_file: path.app_local_data_dir()?.join("queue.json"),
                cache_dir: path.app_cache_dir()?,
                log_dir: path.app_log_dir()?.join("jobs"),
            };
            let config = settings::engine_config(&app.state::<SettingsState>().get());
            let rt = tauri::async_runtime::block_on(async { tokio::runtime::Handle::current() });
            let h = handle.clone();
            let engine = Engine::new(paths, tools, config, rt, move |jobs| {
                on_queue_change(&h, jobs)
            });
            app.manage(engine.clone());

            windows::show_main(&handle).map_err(std::io::Error::other)?;
            tray::build(&handle)?;
            engine.start();
            ipc_server::spawn(&handle, engine);
            bootstrap::spawn(handle.clone());
            updater::spawn_periodic(handle);
            Ok(())
        })
        .on_menu_event(tray::on_menu_event)
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::patch_settings,
            commands::list_jobs,
            commands::probe,
            commands::add_job,
            commands::add_url,
            commands::stop_job,
            commands::remove_job,
            commands::restart_job,
            commands::clear_completed,
            commands::tools_status,
            commands::update_tools,
            commands::update_status,
            commands::check_update,
            commands::install_update,
            commands::open_settings,
            commands::cli_status,
            commands::install_cli,
            commands::uninstall_cli,
        ])
        .build(tauri::generate_context!())
        .expect("error while building kiri")
        .run(|app, event| quit::on_run_event(app, event));
}
