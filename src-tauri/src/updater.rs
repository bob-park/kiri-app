//! GitHub Releases 의 latest.json 으로 새 버전을 확인하고 설치한다.
//! 창이 닫혀도 앱은 메뉴 막대에 살아 있으므로 주기 확인은 Rust 에서 돈다.
use crate::{quit, settings::SettingsState};
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

/// 찾은 업데이트와, 받아 두었으면 그 파일. 메모리에만 둔다(앱을 끄면 다음에 다시 받는다).
struct Pending {
    update: Update,
    bytes: Option<Arc<Vec<u8>>>,
}

#[derive(Default)]
pub struct UpdateState(Mutex<Option<Pending>>);

/// 백그라운드 받기가 도는 중.
static DOWNLOADING: AtomicBool = AtomicBool::new(false);

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct UpdateInfo {
    pub version: String,
    pub notes: String,
    /// 받기를 마쳐 바로 설치할 수 있다.
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

const FIRST_CHECK: Duration = Duration::from_secs(10);
const INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// "큐가 끝나면 재시작" 을 고른 상태.
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
            // 같은 버전을 다시 찾았으면 이미 받은 파일을 버리지 않는다.
            let bytes = old
                .filter(|o| o.update.version == update.version)
                .and_then(|o| o.bytes);
            Pending { update, bytes }
        });
    }
    start_download(app); // DOWNLOADING 을 먼저 켜서 아래 info 가 downloading=true 를 싣는다
    let info = status(app);
    if let Some(i) = &info {
        let _ = app.emit("update-available", i);
    }
    crate::tray::relabel_update(app);
    Ok(info)
}

/// 진행률을 update-progress 로 알리며 받는다. 백그라운드 받기와 설치가 같이 쓴다.
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
                    let _ = app.emit("update-progress", UpdateProgress { received, total });
                }
            },
            || {},
        )
        .await
        .map_err(|e| e.to_string())
}

/// 보관한 업데이트를 아직 받지 않았으면 백그라운드에서 받는다. 이미 받는 중이면 아무것도 하지 않는다.
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
        // 받은 버전이 아직 보관 중인 버전일 때만 결과를 쓴다.
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
        // 결과를 slot 에 쓴 뒤에 끈다 — 그 사이 check 가 ready=false·downloading=false 를 보지 않게.
        // 아래 start_download 재호출보다는 먼저여야 한다.
        DOWNLOADING.store(false, Ordering::SeqCst);
        match outcome {
            Some(Ok(i)) => {
                let _ = app.emit("update-ready", i);
            }
            Some(Err(e)) => {
                eprintln!("kiri update: background download failed: {e}");
                let _ = app.emit("update-download-failed", e);
            }
            // 받는 사이 다른 버전으로 바뀌었으면 이 결과는 버리고 새 버전을 이어서 받는다.
            None => start_download(&app),
        }
    });
}

/// 받고 → 설치 → 재시작. 실패해도 보관한 Update 는 남겨 다시 시도할 수 있다.
pub async fn install(app: &AppHandle) -> Result<(), String> {
    static INSTALLING: AtomicBool = AtomicBool::new(false);
    if INSTALLING.swap(true, Ordering::SeqCst) {
        return Err("busy".into());
    }
    // 지금 설치하므로 남아 있던 "큐가 끝나면" 예약은 버린다.
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
    // 받아 두었으면 그대로, 아니면 지금 받는다. 백그라운드 받기와 드물게 겹치면 한 번 더 받는다(결과는 같음).
    let bytes = match ready {
        Some(b) => b,
        None => Arc::new(download(app, &update).await.map_err(fail)?),
    };
    update.install(bytes.as_slice()).map_err(|e| fail(e.to_string()))?;
    // restart 는 RunEvent::Exit 를 거치지 않을 수 있으므로 여기서 작업을 대기로 되돌리고
    // yt-dlp·ffmpeg 를 정리한다(최대 2초 블로킹이라 블로킹 스레드에서).
    let engine = app.state::<Engine>().inner().clone();
    let _ = tauri::async_runtime::spawn_blocking(move || {
        engine.shutdown_for_exit(Duration::from_secs(2))
    })
    .await;
    quit::allow_exit(); // 재시작이 종료 확인에 막히지 않게
    app.restart()
}

/// after_queue 이고 큐가 남아 있으면 예약만 한다.
/// 예약을 먼저 걸고 큐를 본다: 사이에 마지막 작업이 끝나도 on_queue_change 나
/// 여기 중 정확히 한쪽만 swap 으로 예약을 가져가 설치한다.
pub async fn request_install(app: &AppHandle, after_queue: bool) -> Result<(), String> {
    if after_queue {
        RESTART_AFTER_QUEUE.store(true, Ordering::SeqCst);
        let _ = app.emit("update-scheduled", true);
        if !(app.state::<Engine>().is_idle() && RESTART_AFTER_QUEUE.swap(false, Ordering::SeqCst)) {
            return Ok(());
        }
    }
    install(app).await
}

/// 큐가 바뀔 때마다. 예약돼 있고 큐가 비면 설치한다.
pub fn on_queue_change(app: &AppHandle, jobs: &[Job]) {
    if should_install(RESTART_AFTER_QUEUE.load(Ordering::SeqCst), jobs)
        && RESTART_AFTER_QUEUE.swap(false, Ordering::SeqCst)
    {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = install(&app).await {
                let _ = app.emit("update-error", e);
            }
        });
    }
}

/// setup 에서 한 번. 설정을 매 주기 다시 읽으므로 토글이 바로 반영된다. 오프라인 실패는 로그만.
pub fn spawn_periodic(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FIRST_CHECK).await;
        loop {
            if app.state::<SettingsState>().get().update.auto_check {
                if let Err(e) = check(&app).await {
                    eprintln!("kiri update: check failed: {e}");
                }
            }
            tokio::time::sleep(INTERVAL).await;
        }
    });
}

/// 트레이 항목 하나가 "확인" 과 "설치" 를 겸한다. 설치는 큐가 끝난 뒤로 예약한다.
pub fn on_tray_click(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let r = if status(&app).is_some() {
            request_install(&app, true).await
        } else {
            check(&app).await.map(|_| ())
        };
        if let Err(e) = r {
            let _ = app.emit("update-error", e);
        }
    });
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
}
