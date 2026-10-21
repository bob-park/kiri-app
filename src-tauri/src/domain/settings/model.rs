//! Settings types and their serde shape (snake_case keys, missing fields default).

use serde::{Deserialize, Serialize};

// ponytail: String instead of enums. The frontend TS unions limit the values and
// unknown values are treated as the default.

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub version: u32,
    pub general: General,
    pub download: Download,
    pub update: Update,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct General {
    pub ui_language: String, // system | ko | en
    pub theme: String,       // system | light | dark
    pub close_to_tray: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Download {
    pub dir: String,
    pub quality: String, // best | audio | <N>p
    pub preset: String,  // Preset id
    pub subtitles: Vec<String>,
    pub skip_sheet: bool,
    pub max_concurrent: u8,
    pub hw_accel: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Update {
    pub auto_check: bool,
    /// Unix ms of the last yt-dlp freshness check.
    pub last_ytdlp_check: Option<u64>,
}

pub const UI_LANGUAGES: [&str; 3] = ["system", "ko", "en"];
pub const MAX_CONCURRENT: std::ops::RangeInclusive<u8> = 1..=4;

impl Settings {
    /// Folds stored values the app no longer supports onto supported ones
    /// (e.g. the dropped `ja` language reads as `system`).
    pub fn normalize(&mut self) {
        if !UI_LANGUAGES.contains(&self.general.ui_language.as_str()) {
            self.general.ui_language = "system".into();
        }
        self.download.max_concurrent = self
            .download
            .max_concurrent
            .clamp(*MAX_CONCURRENT.start(), *MAX_CONCURRENT.end());
    }
}

fn default_download_dir() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    format!("{home}/Movies/kiri")
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: 1,
            general: General::default(),
            download: Download::default(),
            update: Update::default(),
        }
    }
}

impl Default for General {
    fn default() -> Self {
        Self {
            ui_language: "system".into(),
            theme: "system".into(),
            close_to_tray: true,
        }
    }
}

impl Default for Download {
    fn default() -> Self {
        Self {
            dir: default_download_dir(),
            quality: "best".into(),
            preset: "original".into(),
            subtitles: vec![],
            skip_sheet: false,
            max_concurrent: 2,
            hw_accel: true,
        }
    }
}

impl Default for Update {
    fn default() -> Self {
        Self {
            auto_check: true,
            last_ytdlp_check: None,
        }
    }
}
