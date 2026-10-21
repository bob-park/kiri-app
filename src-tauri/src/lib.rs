//! Assembly: plugins, setup and the command handler. Logic lives in `app/`, `domain/` and `shared/`.

mod app;
mod domain;
mod shared;

use app::{cli_install, ipc_server, quit, sidecar, state::AppState, tray, updater, windows};
use domain::{queue, settings, tools};
use kiri_core::{
    engine::{Engine, EnginePaths},
    model::Tools,
};
use std::sync::{Arc, Mutex};
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        // Must be first: a second launch only focuses the running app.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Err(e) = windows::show_main(app) {
                log::warn!("cannot show main window for second launch: {e}");
            }
        }))
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let handle = app.handle().clone();
            let path = app.path();
            // ponytail: a DB that cannot be opened aborts setup; add a recovery dialog if
            // users ever hit it.
            let conn = shared::db::open(&path.app_local_data_dir()?.join("kiri.db"))?;
            // A failed import keeps the JSON file in place and starts from defaults.
            match settings::import_json(&conn, &path.app_config_dir()?.join("settings.json")) {
                Ok(true) => log::info!("imported settings.json into the database"),
                Ok(false) => {}
                Err(e) => log::error!("settings.json import failed: {e}"),
            }
            // A failed import keeps queue.json (or queue.json.bad) and makes the engine's
            // first load untrusted, so it leaves partial downloads alone.
            let import_ok =
                match queue::import_json(&conn, &path.app_local_data_dir()?.join("queue.json")) {
                    Ok(imported) => {
                        if imported {
                            log::info!("imported queue.json into the database");
                        }
                        true
                    }
                    Err(e) => {
                        log::error!("queue.json import failed: {e}");
                        false
                    }
                };
            let db = Arc::new(Mutex::new(conn));
            let store = Arc::new(queue::SqliteQueueStore::new(db.clone(), import_ok));
            app.manage(AppState { db });
            app.manage(ipc_server::IpcState::default());
            app.manage(updater::UpdateState::default());

            let bin = path.app_data_dir()?.join("bin");
            let tool_paths = Tools {
                ytdlp: bin.join("yt-dlp"),
                deno: bin.join("deno"),
                ffmpeg: sidecar("ffmpeg"),
                ffprobe: sidecar("ffprobe"),
            };
            app.manage(tools::ToolsState::new(tools::initial_status(&tool_paths)));
            let paths = EnginePaths {
                store,
                cache_dir: path.app_cache_dir()?,
                log_dir: path.app_log_dir()?.join("jobs"),
            };
            let config = settings::engine_config(&app.state::<AppState>().settings());
            let rt = tauri::async_runtime::block_on(async { tokio::runtime::Handle::current() });
            let h = handle.clone();
            let engine = Engine::new(paths, tool_paths, config, rt, move |jobs| {
                queue::commands::on_change(&h, jobs)
            });
            app.manage(engine.clone());

            windows::show_main(&handle).map_err(std::io::Error::other)?;
            tray::build(&handle)?;
            engine.start();
            ipc_server::spawn(&handle, engine);
            tools::commands::spawn(handle.clone());
            updater::spawn_periodic(handle);
            Ok(())
        })
        .on_menu_event(tray::on_menu_event)
        .invoke_handler(tauri::generate_handler![
            settings::commands::get_settings,
            settings::commands::patch_settings,
            queue::commands::list_jobs,
            queue::commands::probe,
            queue::commands::add_job,
            queue::commands::add_url,
            queue::commands::stop_job,
            queue::commands::remove_job,
            queue::commands::restart_job,
            queue::commands::clear_completed,
            tools::commands::tools_status,
            tools::commands::update_tools,
            updater::update_status,
            updater::check_update,
            updater::install_update,
            updater::retry_update_download,
            windows::open_settings,
            cli_install::cli_status,
            cli_install::install_cli,
            cli_install::uninstall_cli,
        ])
        .build(tauri::generate_context!())
        .expect("error while building kiri")
        .run(|app, event| quit::on_run_event(app, event));
}
