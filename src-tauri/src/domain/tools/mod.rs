//! Readiness of the downloaded tools (yt-dlp, Deno). Checked at startup and hourly;
//! yt-dlp itself is updated at most once a day.

pub mod commands;

use kiri_core::model::Tools;
use serde::Serialize;
use std::sync::{Mutex, atomic::AtomicBool};

const DAY_MS: u64 = 24 * 60 * 60 * 1000;

#[derive(Serialize, Clone, Default, Debug, PartialEq)]
pub struct ToolsStatus {
    /// Both yt-dlp and Deno exist on disk. A failed update check never clears it.
    pub ready: bool,
    /// `"yt-dlp"` or `"deno"` while that tool is being downloaded.
    pub installing: Option<String>,
    pub ytdlp_version: Option<String>,
    pub error: Option<String>,
    pub last_check: Option<u64>,
}

#[derive(Default)]
pub struct ToolsState {
    status: Mutex<ToolsStatus>,
    /// A check is running; a second request is dropped.
    busy: AtomicBool,
}

impl ToolsState {
    pub fn new(status: ToolsStatus) -> Self {
        Self {
            status: Mutex::new(status),
            busy: AtomicBool::new(false),
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
}

/// The status before any check: ready as soon as both files exist. Seeded before windows open.
pub fn initial_status(paths: &Tools) -> ToolsStatus {
    ToolsStatus {
        ready: paths.ytdlp.exists() && paths.deno.exists(),
        ..Default::default()
    }
}

/// Whether yt-dlp is due for its daily update check.
pub fn due(last: Option<u64>, now: u64) -> bool {
    last.is_none_or(|l| now.saturating_sub(l) >= DAY_MS)
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
    fn initial_status_ready_when_files_exist() {
        let d = tempfile::tempdir().unwrap();
        let paths = Tools {
            ytdlp: d.path().join("yt-dlp"),
            deno: d.path().join("deno"),
            ffmpeg: d.path().join("ffmpeg"),
            ffprobe: d.path().join("ffprobe"),
        };
        std::fs::write(&paths.ytdlp, b"").unwrap();
        assert_eq!(initial_status(&paths), ToolsStatus::default());
        std::fs::write(&paths.deno, b"").unwrap();
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
        assert_eq!(
            v,
            serde_json::json!({"ready": false, "installing": null, "ytdlp_version": null, "error": null, "last_check": null})
        );
    }
}
