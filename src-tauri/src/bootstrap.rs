//! yt-dlp·Deno 준비 상태. 시작 시 그리고 한 시간마다 확인하고, yt-dlp 는 하루에 한 번 갱신한다.
use crate::settings::SettingsState;
use kiri_core::{
    engine::{Engine, now_ms},
    tools,
};
use serde::Serialize;
use std::{
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tauri::{AppHandle, Emitter, Manager};

const DAY_MS: u64 = 24 * 60 * 60 * 1000;
const RECHECK: Duration = Duration::from_secs(60 * 60);

#[derive(Serialize, Clone, Default, Debug, PartialEq)]
pub struct ToolsStatus {
    pub ready: bool,
    pub installing: bool,
    pub ytdlp_version: Option<String>,
    pub error: Option<String>,
    pub last_check: Option<u64>,
}

#[derive(Default)]
pub struct ToolsState {
    status: Mutex<ToolsStatus>,
    busy: AtomicBool,
}

impl ToolsState {
    pub fn get(&self) -> ToolsStatus {
        self.status.lock().unwrap().clone()
    }
}

pub fn due(last: Option<u64>, now: u64) -> bool {
    last.is_none_or(|l| now.saturating_sub(l) >= DAY_MS)
}

fn update(app: &AppHandle, f: impl FnOnce(&mut ToolsStatus)) {
    let state = app.state::<ToolsState>();
    let snapshot = {
        let mut s = state.status.lock().unwrap();
        f(&mut s);
        s.clone()
    };
    let _ = app.emit("tools-changed", snapshot);
}

/// force: 사용자가 "지금 업데이트" 를 눌렀다. 아니면 yt-dlp 는 하루에 한 번만 확인한다.
pub async fn ensure(app: &AppHandle, force: bool) {
    let state = app.state::<ToolsState>();
    if state.busy.swap(true, Ordering::SeqCst) {
        return;
    }
    let paths = app.state::<Engine>().tools().clone();
    let settings = app.state::<SettingsState>();
    let now = now_ms();
    let need_ytdlp =
        force || !paths.ytdlp.exists() || due(settings.get().update.last_ytdlp_check, now);

    update(app, |s| {
        s.installing = true;
        s.error = None;
    });
    let client = tools::http_client();
    let mut error = tools::ensure_deno(&client, &paths.deno).await.err();
    if need_ytdlp {
        match tools::ensure_ytdlp(&client, &paths.ytdlp).await {
            Ok(_) => {
                let saved = settings.update(app, |s| {
                    s.update.last_ytdlp_check = Some(now);
                    Ok(())
                });
                if let Err(e) = saved {
                    eprintln!("kiri tools: cannot save last check: {e}");
                }
            }
            Err(e) => error = error.or(Some(e)),
        }
    }
    let version = tools::version(&paths.ytdlp).await;
    let last_check = settings.get().update.last_ytdlp_check;
    let ready = paths.ytdlp.exists() && paths.deno.exists();
    update(app, |s| {
        *s = ToolsStatus {
            ready,
            installing: false,
            ytdlp_version: version,
            error,
            last_check,
        };
    });
    state.busy.store(false, Ordering::SeqCst);
}

pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            ensure(&app, false).await;
            tokio::time::sleep(RECHECK).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn due_after_a_day() {
        assert!(due(None, 0));
        assert!(!due(Some(1_000), 1_000 + DAY_MS - 1));
        assert!(due(Some(1_000), 1_000 + DAY_MS));
    }

    #[test]
    fn status_json_shape() {
        let v = serde_json::to_value(ToolsStatus::default()).unwrap();
        assert_eq!(
            v,
            serde_json::json!({"ready": false, "installing": false, "ytdlp_version": null, "error": null, "last_check": null})
        );
    }
}
