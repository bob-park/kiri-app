//! Checks GitHub Releases `latest.json` for a new version, downloads and installs it.
//! The app keeps running in the menu bar with no window, so the periodic check runs in Rust.
use crate::app::{quit, state::AppState, tray};
use kiri_core::{engine::Engine, model::Job};
use serde::Serialize;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

/// The update found and, once downloaded, its bytes. Memory only (a restart downloads again).
struct Pending {
    update: Update,
    bytes: Option<Arc<Vec<u8>>>,
}

#[derive(Default)]
pub struct UpdateState(Mutex<Option<Pending>>);

/// A background download is running.
static DOWNLOADING: AtomicBool = AtomicBool::new(false);

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct UpdateInfo {
    pub version: String,
    pub notes: String,
    /// Downloaded; can be installed right away.
    pub ready: bool,
    pub downloading: bool,
}

impl UpdateInfo {
    fn from_pending(p: &Pending) -> Self {
        Self {
            version: p.update.version.clone(),
            notes: p.update.body.clone().unwrap_or_default(),
            ready: p.bytes.is_some(),
            downloading: DOWNLOADING.load(Ordering::SeqCst),
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct UpdateProgress {
    pub received: u64,
    pub total: Option<u64>,
}

/// Payload: `UpdateStatus`. One event for every updater change.
pub const UPDATE_STATUS: &str = "update://status";

/// What changed, tagged by `kind`; struct payloads are flattened next to it.
#[derive(Serialize, Clone, Debug)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UpdateStatus {
    /// A check found an update.
    Available(UpdateInfo),
    Progress(UpdateProgress),
    /// The background download finished; the update can be installed right away.
    Ready(UpdateInfo),
    DownloadFailed {
        error: String,
    },
    /// "Restart when the queue finishes" was chosen.
    Scheduled,
    /// Installing (or the tray check) failed.
    Error {
        error: String,
    },
}

fn emit(app: &AppHandle, status: UpdateStatus) {
    let _ = app.emit(UPDATE_STATUS, status);
}

const FIRST_CHECK: Duration = Duration::from_secs(10);
const INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// "Restart when the queue finishes" is scheduled.
static RESTART_AFTER_QUEUE: AtomicBool = AtomicBool::new(false);

pub fn should_install(pending: bool, jobs: &[Job]) -> bool {
    pending && !jobs.iter().any(|j| j.state.is_pending())
}

pub fn status(app: &AppHandle) -> Option<UpdateInfo> {
    app.try_state::<UpdateState>()?
        .0
        .lock()
        .unwrap()
        .as_ref()
        .map(UpdateInfo::from_pending)
}

pub async fn check(app: &AppHandle) -> Result<Option<UpdateInfo>, String> {
    let found = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?;
    {
        let state = app.state::<UpdateState>();
        let mut slot = state.0.lock().unwrap();
        let old = slot.take();
        *slot = found.map(|update| {
            // Found the same version again: keep the bytes already downloaded.
            let bytes = old
                .filter(|o| o.update.version == update.version)
                .and_then(|o| o.bytes);
            Pending { update, bytes }
        });
    }
    start_download(app); // sets DOWNLOADING first so the info below carries downloading=true
    let info = status(app);
    if let Some(i) = &info {
        emit(app, UpdateStatus::Available(i.clone()));
    }
    tray::relabel_update(app);
    Ok(info)
}

/// Downloads while reporting `Progress`. Shared by the background download and install.
async fn download(app: &AppHandle, update: &Update) -> Result<Vec<u8>, String> {
    let mut received: u64 = 0;
    let mut last_pct = u64::MAX;
    update
        .download(
            |chunk, total| {
                received += chunk as u64;
                let pct = total.map_or(0, |t| received * 100 / t.max(1));
                if pct != last_pct {
                    last_pct = pct;
                    emit(
                        app,
                        UpdateStatus::Progress(UpdateProgress { received, total }),
                    );
                }
            },
            || {},
        )
        .await
        .map_err(|e| e.to_string())
}

/// Downloads the kept update in the background unless it is downloaded or already downloading.
pub fn start_download(app: &AppHandle) {
    let update = {
        let state = app.state::<UpdateState>();
        let slot = state.0.lock().unwrap();
        match slot.as_ref() {
            Some(p) if p.bytes.is_none() => p.update.clone(),
            _ => return,
        }
    };
    if DOWNLOADING.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let result = download(&app, &update).await;
        // Store the result only if the downloaded version is still the kept one.
        let outcome = {
            let state = app.state::<UpdateState>();
            let mut slot = state.0.lock().unwrap();
            match slot.as_mut() {
                Some(p) if p.update.version == update.version => Some(result.map(|bytes| {
                    p.bytes = Some(Arc::new(bytes));
                    UpdateInfo::from_pending(p)
                })),
                _ => None,
            }
        };
        // Clear the flag after writing the slot, so a check in between never sees
        // ready=false and downloading=false. It must also precede the start_download retry below.
        DOWNLOADING.store(false, Ordering::SeqCst);
        match outcome {
            Some(Ok(i)) => emit(&app, UpdateStatus::Ready(i)),
            Some(Err(error)) => {
                log::warn!("update background download failed: {error}");
                emit(&app, UpdateStatus::DownloadFailed { error });
            }
            // The kept version changed meanwhile: drop this result and download the new one.
            None => start_download(&app),
        }
    });
}

/// Download → install → restart. On failure the kept update stays, so it can be retried.
pub async fn install(app: &AppHandle) -> Result<(), String> {
    static INSTALLING: AtomicBool = AtomicBool::new(false);
    if INSTALLING.swap(true, Ordering::SeqCst) {
        return Err("busy".into());
    }
    // Installing now, so drop any pending "after the queue" schedule.
    RESTART_AFTER_QUEUE.store(false, Ordering::SeqCst);
    let fail = |e: String| {
        INSTALLING.store(false, Ordering::SeqCst);
        e
    };
    let (update, ready) = app
        .state::<UpdateState>()
        .0
        .lock()
        .unwrap()
        .as_ref()
        .map(|p| (p.update.clone(), p.bytes.clone()))
        .ok_or_else(|| fail("error.no_update".to_string()))?;
    // Use the downloaded bytes, or download now. A rare overlap with the background
    // download just downloads twice (same result).
    let bytes = match ready {
        Some(b) => b,
        None => Arc::new(download(app, &update).await.map_err(fail)?),
    };
    update
        .install(bytes.as_slice())
        .map_err(|e| fail(e.to_string()))?;
    // restart may skip RunEvent::Exit, so requeue jobs and stop yt-dlp/ffmpeg here
    // (blocks up to 2 s, hence the blocking thread).
    let engine = app.state::<Engine>().inner().clone();
    let _ = tauri::async_runtime::spawn_blocking(move || {
        engine.shutdown_for_exit(Duration::from_secs(2))
    })
    .await;
    quit::allow_exit(); // so the quit confirmation never blocks the restart
    app.restart()
}

/// With `after_queue` and work still queued, only schedules the install.
/// The schedule is set before the queue is checked: if the last job finishes in between,
/// exactly one of `on_queue_change` and this function takes the schedule (swap) and installs.
pub async fn request_install(app: &AppHandle, after_queue: bool) -> Result<(), String> {
    if after_queue {
        RESTART_AFTER_QUEUE.store(true, Ordering::SeqCst);
        emit(app, UpdateStatus::Scheduled);
        if !(app.state::<Engine>().is_idle() && RESTART_AFTER_QUEUE.swap(false, Ordering::SeqCst)) {
            return Ok(());
        }
    }
    install(app).await
}

/// On every queue change: installs if scheduled and the queue is idle.
pub fn on_queue_change(app: &AppHandle, jobs: &[Job]) {
    if should_install(RESTART_AFTER_QUEUE.load(Ordering::SeqCst), jobs)
        && RESTART_AFTER_QUEUE.swap(false, Ordering::SeqCst)
    {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(error) = install(&app).await {
                emit(&app, UpdateStatus::Error { error });
            }
        });
    }
}

/// Called once from setup. Settings are re-read every round, so the toggle applies at once.
/// Failures (e.g. offline) are only logged.
pub fn spawn_periodic(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FIRST_CHECK).await;
        loop {
            if app.state::<AppState>().settings().update.auto_check {
                if let Err(e) = check(&app).await {
                    log::warn!("update check failed: {e}");
                }
            }
            tokio::time::sleep(INTERVAL).await;
        }
    });
}

/// One tray item is both "check" and "install". The install waits for the queue.
pub fn on_tray_click(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let r = if status(&app).is_some() {
            request_install(&app, true).await
        } else {
            check(&app).await.map(|_| ())
        };
        if let Err(error) = r {
            emit(&app, UpdateStatus::Error { error });
        }
    });
}

#[tauri::command]
pub fn update_status(app: AppHandle) -> Option<UpdateInfo> {
    status(&app)
}

#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<Option<UpdateInfo>, String> {
    check(&app).await
}

#[tauri::command]
pub async fn install_update(app: AppHandle, after_queue: bool) -> Result<(), String> {
    request_install(&app, after_queue).await
}

#[tauri::command]
pub fn retry_update_download(app: AppHandle) {
    start_download(&app);
}

#[cfg(test)]
mod tests {
    use super::*;
    use kiri_core::model::{JobOptions, JobSource, JobState, Preset};

    fn job(state: JobState) -> Job {
        Job {
            id: 1,
            source: JobSource::Youtube { url: "u".into() },
            title: "t".into(),
            thumbnail: None,
            duration_secs: None,
            quality_label: "720p".into(),
            options: JobOptions {
                format_id: None,
                preset: Preset::Original,
                subtitles: vec![],
                auto_subtitles: false,
                max_height: None,
            },
            state,
            progress: 0.0,
            speed: None,
            eta: None,
            output: None,
            work_dir: None,
            original: None,
            created_at: 0,
        }
    }

    #[test]
    fn installs_after_queue_only_when_pending_and_idle() {
        assert!(!should_install(false, &[]));
        assert!(should_install(
            true,
            &[job(JobState::Completed), job(JobState::Failed("x".into()))]
        ));
        assert!(!should_install(true, &[job(JobState::Queued)]));
        assert!(!should_install(true, &[job(JobState::Encoding)]));
    }

    #[test]
    fn info_serializes_with_frontend_keys() {
        let v = serde_json::to_value(UpdateInfo {
            version: "0.2.0".into(),
            notes: "fix".into(),
            ready: true,
            downloading: false,
        })
        .unwrap();
        assert_eq!(
            v,
            serde_json::json!({"version": "0.2.0", "notes": "fix", "ready": true, "downloading": false})
        );
        let p = serde_json::to_value(UpdateProgress {
            received: 5,
            total: None,
        })
        .unwrap();
        assert_eq!(p, serde_json::json!({"received": 5, "total": null}));
    }

    #[test]
    fn update_status_json_shape() {
        let info = || UpdateInfo {
            version: "0.2.0".into(),
            notes: "fix".into(),
            ready: true,
            downloading: false,
        };
        let cases = [
            (
                UpdateStatus::Available(info()),
                serde_json::json!({"kind": "available", "version": "0.2.0", "notes": "fix", "ready": true, "downloading": false}),
            ),
            (
                UpdateStatus::Progress(UpdateProgress {
                    received: 5,
                    total: Some(10),
                }),
                serde_json::json!({"kind": "progress", "received": 5, "total": 10}),
            ),
            (
                UpdateStatus::Ready(info()),
                serde_json::json!({"kind": "ready", "version": "0.2.0", "notes": "fix", "ready": true, "downloading": false}),
            ),
            (
                UpdateStatus::DownloadFailed {
                    error: "net".into(),
                },
                serde_json::json!({"kind": "download_failed", "error": "net"}),
            ),
            (
                UpdateStatus::Scheduled,
                serde_json::json!({"kind": "scheduled"}),
            ),
            (
                UpdateStatus::Error {
                    error: "busy".into(),
                },
                serde_json::json!({"kind": "error", "error": "busy"}),
            ),
        ];
        for (status, want) in cases {
            assert_eq!(serde_json::to_value(status).unwrap(), want);
        }
    }
}
