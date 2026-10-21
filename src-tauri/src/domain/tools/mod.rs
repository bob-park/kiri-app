//! Readiness of the downloaded tools (yt-dlp, Deno). Checked at startup and hourly;
//! yt-dlp itself is updated at most once a day.

pub mod commands;

use serde::Serialize;
use std::sync::{Mutex, atomic::AtomicBool};

const DAY_MS: u64 = 24 * 60 * 60 * 1000;

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
    /// A check is running; a second request is dropped.
    busy: AtomicBool,
}

impl ToolsState {
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
    fn status_json_shape() {
        let v = serde_json::to_value(ToolsStatus::default()).unwrap();
        assert_eq!(
            v,
            serde_json::json!({"ready": false, "installing": false, "ytdlp_version": null, "error": null, "last_check": null})
        );
    }
}
