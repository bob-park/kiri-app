//! Tauri adapters for the tools domain: the commands, the `tools://changed` event and
//! the background check loop.

use super::{ToolsState, ToolsStatus, due};
use crate::{app::state::AppState, domain::settings, shared::error::AppResult};
use kiri_core::{
    engine::{Engine, now_ms},
    tools,
};
use std::{sync::atomic::Ordering, time::Duration};
use tauri::{AppHandle, Emitter, Manager, State};

/// Payload: the new `ToolsStatus`.
pub const TOOLS_CHANGED: &str = "tools://changed";

const RECHECK: Duration = Duration::from_secs(60 * 60);

#[tauri::command]
pub fn tools_status(state: State<'_, ToolsState>) -> AppResult<ToolsStatus> {
    Ok(state.get())
}

#[tauri::command]
pub async fn update_tools(app: AppHandle) -> AppResult<()> {
    ensure(&app, true).await;
    Ok(())
}

fn update(app: &AppHandle, f: impl FnOnce(&mut ToolsStatus)) {
    let snapshot = app.state::<ToolsState>().update(f);
    let _ = app.emit(TOOLS_CHANGED, snapshot);
}

/// `force`: the user pressed "Update now". Otherwise yt-dlp is checked once a day.
/// `ready` follows the files on disk, so a failed check never hides tools that exist.
pub async fn ensure(app: &AppHandle, force: bool) {
    let state = app.state::<ToolsState>();
    if state.busy.swap(true, Ordering::SeqCst) {
        return;
    }
    let paths = app.state::<Engine>().tools().clone();
    let now = now_ms();
    let client = tools::http_client();
    let mut error = None;

    if !paths.deno.exists() {
        installing(app, "deno");
        error = tools::ensure_deno(&client, &paths.deno).await.err();
    }
    let need_ytdlp = force
        || !paths.ytdlp.exists()
        || due(
            app.state::<AppState>().settings().update.last_ytdlp_check,
            now,
        );
    if need_ytdlp {
        // The hash check runs without touching `installing`; only a real download sets it.
        let result = async {
            let want = tools::ytdlp_latest_hash(&client).await?;
            if tools::ytdlp_matches(&paths.ytdlp, &want) {
                return Ok(());
            }
            installing(app, "yt-dlp");
            tools::install_ytdlp(&client, &paths.ytdlp, &want).await
        }
        .await;
        if let Err(e) = result {
            error = error.or(Some(e));
        } else {
            let patch = serde_json::json!({ "update": { "last_ytdlp_check": now } });
            if let Err(e) = settings::commands::patch_and_notify(app, &patch) {
                log::warn!("cannot save last yt-dlp check: {e}");
            }
        }
    }

    let version = tools::version(&paths.ytdlp).await;
    let last_check = app.state::<AppState>().settings().update.last_ytdlp_check;
    let ready = paths.ytdlp.exists() && paths.deno.exists();
    update(app, |s| {
        *s = ToolsStatus {
            ready,
            installing: None,
            ytdlp_version: version,
            error,
            last_check,
        };
    });
    state.busy.store(false, Ordering::SeqCst);
}

/// Marks `name` as downloading, right before the download starts.
fn installing(app: &AppHandle, name: &str) {
    update(app, |s| {
        s.installing = Some(name.into());
        s.error = None;
    });
}

/// Runs `ensure` at startup and then every hour.
pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            ensure(&app, false).await;
            tokio::time::sleep(RECHECK).await;
        }
    });
}
