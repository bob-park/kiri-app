//! Tauri adapters for the tools domain: the commands, the `tools://changed` event and the
//! startup install. Every install and check goes through `ToolsState::try_busy`.

use super::{ToolsState, ToolsStatus, all_installed, check_due};
use crate::{domain::settings, shared::error::AppResult};
use kiri_core::{
    engine::{Engine, now_ms},
    model::Tools,
    tools::{self, ALL_TOOLS, Release, ToolName},
};
use tauri::{AppHandle, Emitter, Manager, State};

/// Payload: the new `ToolsStatus`.
pub const TOOLS_CHANGED: &str = "tools://changed";

#[tauri::command]
pub fn tools_status(state: State<'_, ToolsState>) -> AppResult<ToolsStatus> {
    Ok(state.get())
}

/// Installs every missing tool. Runs at startup and from the failed-toast retry.
#[tauri::command]
pub async fn install_tools(app: AppHandle) -> AppResult<()> {
    let state = app.state::<ToolsState>();
    let Some(_busy) = state.try_busy() else {
        return Ok(());
    };
    let paths = app.state::<Engine>().tools().clone();
    let missing: Vec<_> = ALL_TOOLS
        .into_iter()
        .filter(|&t| !tools::is_installed(t, &paths))
        .collect();
    install_all(&app, &paths, &missing).await;
    Ok(())
}

/// Installs `names`, each from the release the last check resolved (or the latest one).
/// Returns the status afterwards, or `None` when another install or check was running.
#[tauri::command]
pub async fn update_tools(app: AppHandle, names: Vec<ToolName>) -> AppResult<Option<ToolsStatus>> {
    let state = app.state::<ToolsState>();
    let Some(_busy) = state.try_busy() else {
        return Ok(None);
    };
    let paths = app.state::<Engine>().tools().clone();
    install_all(&app, &paths, &names).await;
    Ok(Some(state.get()))
}

/// Resolves the latest release of every tool. A failed check is silent and keeps the old `latest`.
#[tauri::command]
pub async fn check_tool_updates(app: AppHandle) -> AppResult<()> {
    let state = app.state::<ToolsState>();
    let Some(_busy) = state.try_busy() else {
        return Ok(());
    };
    let client = tools::http_client();
    let (y, f, d) = tokio::join!(
        tools::latest(&client, ToolName::Ytdlp),
        tools::latest(&client, ToolName::Ffmpeg),
        tools::latest(&client, ToolName::Deno),
    );
    let now = now_ms();
    if y.is_ok() {
        *state.checked_at.lock().unwrap() = Some(now); // counts toward the hourly yt-dlp check
    }
    let mut any = false;
    for (tool, result) in [
        (ToolName::Ytdlp, y),
        (ToolName::Ffmpeg, f),
        (ToolName::Deno, d),
    ] {
        match result {
            Ok(r) => {
                record(&app, tool, r);
                any = true;
            }
            Err(e) => log::info!("{tool:?} update check failed: {e}"),
        }
    }
    // Offline: keep the last check time, so it does not claim a check that never happened.
    if any {
        let patch = serde_json::json!({ "update": { "last_ytdlp_check": now } });
        if let Err(e) = settings::commands::patch_and_notify(&app, &patch) {
            log::warn!("cannot save last tools check: {e}");
        }
        update(&app, |s| s.last_check = Some(now));
    }
    Ok(())
}

/// The newer yt-dlp version, if any. Hits the network at most hourly; otherwise it reuses
/// the last result.
#[tauri::command]
pub async fn check_ytdlp_update(app: AppHandle) -> AppResult<Option<String>> {
    let state = app.state::<ToolsState>();
    let now = now_ms();
    if check_due(*state.checked_at.lock().unwrap(), now)
        && let Some(_busy) = state.try_busy()
    {
        match tools::latest(&tools::http_client(), ToolName::Ytdlp).await {
            Ok(r) => {
                *state.checked_at.lock().unwrap() = Some(now);
                record(&app, ToolName::Ytdlp, r);
            }
            Err(e) => log::info!("yt-dlp update check failed: {e}"),
        }
    }
    let info = state.get().ytdlp;
    Ok(info.latest.clone().filter(|_| info.has_update()))
}

/// Stores a resolved release and shows its version as `latest`.
fn record(app: &AppHandle, tool: ToolName, release: Release) {
    let version = release.version.clone();
    let state = app.state::<ToolsState>();
    state.releases.lock().unwrap().insert(tool, release);
    update(app, |s| s.info_mut(tool).latest = Some(version));
}

/// Installs `names` in order (the caller holds `busy`), then refreshes versions and `ready`
/// and kicks the engine.
/// Each tool installs the release a check stored, or the latest one if none is stored.
async fn install_all(app: &AppHandle, paths: &Tools, names: &[ToolName]) {
    let state = app.state::<ToolsState>();
    let client = tools::http_client();
    let mut error = None;
    for &tool in names {
        update(app, |s| {
            s.installing = Some(tool);
            s.error = None;
        });
        let stored = state.releases.lock().unwrap().get(&tool).cloned();
        let result = async {
            let release = match stored {
                Some(r) => r,
                None => tools::latest(&client, tool).await?,
            };
            tools::install(&client, tool, &release, paths).await?;
            Ok::<_, String>(release)
        }
        .await;
        match result {
            Ok(r) => record(app, tool, r),
            Err(e) => {
                log::warn!("{tool:?} install failed: {e}");
                error = error.or(Some(e));
            }
        }
    }
    let mut current = Vec::with_capacity(ALL_TOOLS.len());
    for tool in ALL_TOOLS {
        current.push((tool, tools::installed_version(tool, paths).await));
    }
    let ready = all_installed(paths);
    update(app, |s| {
        for (tool, v) in current {
            s.info_mut(tool).current = v;
        }
        s.ready = ready;
        s.installing = None;
        s.error = error;
    });
    app.state::<Engine>().start(); // runs queued jobs held while a tool was missing
}

fn update(app: &AppHandle, f: impl FnOnce(&mut ToolsStatus)) {
    let snapshot = app.state::<ToolsState>().update(f);
    let _ = app.emit(TOOLS_CHANGED, snapshot);
}

/// Installs whatever is missing, once, in the background. Updates are never checked here.
pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let _ = install_tools(app).await;
    });
}
