//! Downloaded tools (yt-dlp, FFmpeg, Deno): installed at startup when missing, and checked
//! and updated only on request (Settings → Updates, or the yt-dlp hint).

pub mod commands;

use kiri_core::{
    model::Tools,
    tools::{self, ALL_TOOLS, Progress, Release, ToolName},
};
use serde::Serialize;
use std::{
    collections::HashMap,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

const HOUR_MS: u64 = 60 * 60 * 1000;
/// Download progress events go out at most this often (plus the final byte).
const PROGRESS_EVERY_MS: u64 = 100;

#[derive(Serialize, Clone, Default, Debug, PartialEq)]
pub struct ToolInfo {
    /// The installed version; `None` while missing.
    pub current: Option<String>,
    /// The newest release seen this session; `None` until a check succeeds.
    pub latest: Option<String>,
}

impl ToolInfo {
    pub fn has_update(&self) -> bool {
        self.latest.is_some() && self.latest != self.current
    }
}

#[derive(Serialize, Clone, Default, Debug, PartialEq)]
pub struct ToolsStatus {
    /// ffmpeg, ffprobe, yt-dlp and Deno all exist on disk. A failed check never clears it.
    pub ready: bool,
    /// The tool being downloaded.
    pub installing: Option<ToolName>,
    /// Bytes of `installing` received so far; `None` when nothing downloads.
    pub progress: Option<Progress>,
    pub ytdlp: ToolInfo,
    pub ffmpeg: ToolInfo,
    pub deno: ToolInfo,
    /// The last failed install. A failed version check stays silent.
    pub error: Option<String>,
    /// `settings.update.last_ytdlp_check`: the last check of all tools.
    pub last_check: Option<u64>,
}

impl ToolsStatus {
    pub fn info_mut(&mut self, tool: ToolName) -> &mut ToolInfo {
        match tool {
            ToolName::Ytdlp => &mut self.ytdlp,
            ToolName::Ffmpeg => &mut self.ffmpeg,
            ToolName::Deno => &mut self.deno,
        }
    }
}

#[derive(Default)]
pub struct ToolsState {
    status: Mutex<ToolsStatus>,
    /// An install or check is running; a second request returns at once.
    /// Installs must stay serialized: their staging dirs are per tool.
    busy: AtomicBool,
    /// Releases resolved by the last checks, so an update installs exactly what was shown.
    releases: Mutex<HashMap<ToolName, Release>>,
    /// The last successful yt-dlp check this session (in memory only).
    checked_at: Mutex<Option<u64>>,
}

/// Clears `busy` when dropped, so an early return never leaves the tools locked.
struct Busy<'a>(&'a AtomicBool);

impl Drop for Busy<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

impl ToolsState {
    pub fn new(status: ToolsStatus) -> Self {
        Self {
            status: Mutex::new(status),
            ..Default::default()
        }
    }

    pub fn get(&self) -> ToolsStatus {
        self.status.lock().unwrap().clone()
    }

    /// Applies `f` and returns the new snapshot.
    pub fn update(&self, f: impl FnOnce(&mut ToolsStatus)) -> ToolsStatus {
        let mut s = self.status.lock().unwrap();
        f(&mut s);
        s.clone()
    }

    /// `None` while another install or check runs.
    fn try_busy(&self) -> Option<Busy<'_>> {
        (!self.busy.swap(true, Ordering::SeqCst)).then_some(Busy(&self.busy))
    }
}

pub fn all_installed(paths: &Tools) -> bool {
    ALL_TOOLS.iter().all(|&t| tools::is_installed(t, paths))
}

/// The status before any install: ready as soon as every file exists. Seeded before windows open.
pub fn initial_status(paths: &Tools) -> ToolsStatus {
    ToolsStatus {
        ready: all_installed(paths),
        ..Default::default()
    }
}

/// Whether the yt-dlp-only check may hit the network again (at most hourly).
pub fn check_due(checked_at: Option<u64>, now: u64) -> bool {
    checked_at.is_none_or(|c| now.saturating_sub(c) >= HOUR_MS)
}

/// Whether a download progress update is worth an event: the first one, the last byte,
/// or `PROGRESS_EVERY_MS` since the last event.
pub fn progress_due(last_ms: Option<u64>, now: u64, p: Progress) -> bool {
    last_ms.is_none_or(|l| now.saturating_sub(l) >= PROGRESS_EVERY_MS) || p.total == Some(p.done)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: u64 = 60 * 1000;

    #[test]
    fn check_due_after_an_hour() {
        assert!(check_due(None, 0));
        assert!(!check_due(Some(1_000), 1_000 + 59 * MIN));
        assert!(check_due(Some(1_000), 1_000 + 60 * MIN));
    }

    #[test]
    fn progress_due_throttles_but_keeps_the_last_byte() {
        let p = |done, total| Progress { done, total };
        assert!(progress_due(None, 0, p(0, Some(10))));
        assert!(!progress_due(Some(1_000), 1_099, p(5, Some(10))));
        assert!(progress_due(Some(1_000), 1_100, p(5, Some(10))));
        assert!(progress_due(Some(1_000), 1_001, p(10, Some(10))));
        assert!(!progress_due(Some(1_000), 1_001, p(10, None)));
    }

    #[test]
    fn has_update_needs_a_different_latest() {
        let info = |c: Option<&str>, l: Option<&str>| ToolInfo {
            current: c.map(Into::into),
            latest: l.map(Into::into),
        };
        assert!(info(Some("9.0.1"), Some("9.0.2")).has_update());
        assert!(!info(Some("9.0.2"), Some("9.0.2")).has_update());
        assert!(!info(Some("9.0.1"), None).has_update()); // offline
        assert!(info(None, Some("9.0.2")).has_update());
    }

    #[test]
    fn initial_status_ready_only_with_all_four() {
        let d = tempfile::tempdir().unwrap();
        let paths = Tools {
            ytdlp: d.path().join("yt-dlp"),
            deno: d.path().join("deno"),
            ffmpeg: d.path().join("ffmpeg"),
            ffprobe: d.path().join("ffprobe"),
        };
        for p in [&paths.ytdlp, &paths.deno, &paths.ffmpeg] {
            std::fs::write(p, b"").unwrap();
        }
        assert_eq!(initial_status(&paths), ToolsStatus::default()); // ffprobe missing
        std::fs::write(&paths.ffprobe, b"").unwrap();
        assert_eq!(
            initial_status(&paths),
            ToolsStatus {
                ready: true,
                ..Default::default()
            }
        );
    }

    #[test]
    fn status_json_shape() {
        let v = serde_json::to_value(ToolsStatus::default()).unwrap();
        let info = serde_json::json!({"current": null, "latest": null});
        assert_eq!(
            v,
            serde_json::json!({
                "ready": false,
                "installing": null,
                "progress": null,
                "ytdlp": info,
                "ffmpeg": info,
                "deno": info,
                "error": null,
                "last_check": null
            })
        );
    }
}
