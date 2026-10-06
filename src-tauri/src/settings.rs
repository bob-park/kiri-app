use kiri_core::{engine::EngineConfig, model::Preset};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::Mutex,
};
use tauri::{AppHandle, Emitter};

/// patch 를 base 에 깊게 병합한다. 객체끼리는 재귀, 그 외(배열 포함)는 통째로 치환.
pub fn merge(base: &mut Value, patch: &Value) {
    match (base, patch) {
        (Value::Object(b), Value::Object(p)) => {
            for (k, v) in p {
                merge(b.entry(k.clone()).or_insert(Value::Null), v);
            }
        }
        (b, p) => *b = p.clone(),
    }
}

// ponytail: 열거형 대신 String. 프론트 TS 유니온이 값을 제한하고 모르는 값은 기본값으로 취급한다.

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
    pub ui_language: String, // system | ko | en | ja
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
    /// unix ms. yt-dlp 최신 여부를 마지막으로 확인한 시각.
    pub last_ytdlp_check: Option<u64>,
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

impl Settings {
    pub fn load(path: &Path) -> Settings {
        match fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
                eprintln!("kiri settings: parse error, using defaults: {e}");
                Settings::default()
            }),
            Err(e) => {
                if e.kind() != io::ErrorKind::NotFound {
                    eprintln!("kiri settings: read error, using defaults: {e}");
                }
                Settings::default()
            }
        }
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        fs::write(
            &tmp,
            serde_json::to_string_pretty(self).map_err(io::Error::other)?,
        )?;
        fs::rename(tmp, path)
    }
}

pub fn engine_config(s: &Settings) -> EngineConfig {
    EngineConfig {
        max_concurrent: s.download.max_concurrent.clamp(1, 4) as usize,
        hw_accel: s.download.hw_accel,
        download_dir: PathBuf::from(&s.download.dir),
        default_quality: s.download.quality.clone(),
        default_preset: s.download.preset.parse().unwrap_or(Preset::Original),
        default_subtitles: s.download.subtitles.clone(),
    }
}

/// current 에 patch 를 깊게 병합한 새 설정. 형식이 틀리면 Err.
pub fn apply_patch(current: &Settings, patch: &Value) -> Result<Settings, String> {
    let mut v = serde_json::to_value(current).map_err(|e| e.to_string())?;
    merge(&mut v, patch);
    serde_json::from_value(v).map_err(|e| e.to_string())
}

pub struct SettingsState {
    path: PathBuf,
    current: Mutex<Settings>,
}

impl SettingsState {
    pub fn new(path: PathBuf) -> Self {
        let current = Mutex::new(Settings::load(&path));
        Self { path, current }
    }

    pub fn get(&self) -> Settings {
        self.current.lock().unwrap().clone()
    }

    /// 잠금을 쥔 채 복사 → f → 저장 → 교체해서 동시 쓰기가 서로를 덮지 않게 한다.
    /// f 가 Err 면 저장하지 않는다. settings-changed 는 잠금을 놓은 뒤 보낸다.
    pub fn update(
        &self,
        app: &AppHandle,
        f: impl FnOnce(&mut Settings) -> Result<(), String>,
    ) -> Result<Settings, String> {
        let next = {
            let mut cur = self.current.lock().unwrap();
            let mut next = cur.clone();
            f(&mut next)?;
            next.save(&self.path).map_err(|e| e.to_string())?;
            *cur = next.clone();
            next
        };
        app.emit("settings-changed", &next)
            .map_err(|e| e.to_string())?;
        crate::tray::relabel(app, &next); // 트레이가 아직 없으면 무시된다
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_spec() {
        let s = Settings::default();
        assert_eq!(s.version, 1);
        assert_eq!(
            (s.general.ui_language.as_str(), s.general.theme.as_str()),
            ("system", "system")
        );
        assert!(s.general.close_to_tray);
        assert!(
            s.download.dir.ends_with("/Movies/kiri"),
            "{}",
            s.download.dir
        );
        assert_eq!(
            (s.download.quality.as_str(), s.download.preset.as_str()),
            ("best", "original")
        );
        assert!(s.download.subtitles.is_empty());
        assert!(!s.download.skip_sheet);
        assert_eq!(s.download.max_concurrent, 2);
        assert!(s.download.hw_accel);
        assert!(s.update.auto_check);
        assert_eq!(s.update.last_ytdlp_check, None);
    }

    #[test]
    fn roundtrip_uses_snake_case_keys() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("settings.json");
        let mut s = Settings::default();
        s.download.max_concurrent = 3;
        s.general.close_to_tray = false;
        s.save(&path).unwrap();
        assert_eq!(Settings::load(&path), s);
        let text = fs::read_to_string(&path).unwrap();
        assert!(
            text.contains("\"close_to_tray\"") && text.contains("\"max_concurrent\""),
            "{text}"
        );
    }

    #[test]
    fn missing_fields_and_corrupt_file() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("settings.json");
        fs::write(&path, r#"{"general":{"theme":"dark"}}"#).unwrap();
        let s = Settings::load(&path);
        assert_eq!(s.general.theme, "dark");
        assert_eq!(s.download.max_concurrent, 2);
        fs::write(&path, "{nope").unwrap();
        assert_eq!(Settings::load(&path), Settings::default());
    }

    #[test]
    fn merge_recurses_and_replaces() {
        let mut base = serde_json::json!({"general": {"theme": "system", "ui_language": "system"}, "download": {"subtitles": ["ko"]}});
        merge(
            &mut base,
            &serde_json::json!({"general": {"theme": "dark"}, "download": {"subtitles": []}}),
        );
        assert_eq!(base["general"]["theme"], "dark");
        assert_eq!(base["general"]["ui_language"], "system");
        assert_eq!(base["download"]["subtitles"], serde_json::json!([]));
    }

    #[test]
    fn apply_patch_merges_and_rejects_bad_types() {
        let mut cur = Settings::default();
        cur.update.last_ytdlp_check = Some(7);
        let next = apply_patch(
            &cur,
            &serde_json::json!({"download": {"max_concurrent": 3}, "general": {"theme": "dark"}}),
        )
        .unwrap();
        assert_eq!(next.download.max_concurrent, 3);
        assert_eq!(next.general.theme, "dark");
        assert_eq!(next.update.last_ytdlp_check, Some(7));
        assert_eq!(next.download.dir, cur.download.dir);
        assert!(
            apply_patch(
                &cur,
                &serde_json::json!({"download": {"max_concurrent": "x"}})
            )
            .is_err()
        );
    }

    #[test]
    fn engine_config_maps_and_clamps() {
        let mut s = Settings::default();
        s.download.max_concurrent = 9;
        s.download.preset = "mp4-hevc".into();
        s.download.subtitles = vec!["ko".into()];
        let c = engine_config(&s);
        assert_eq!(c.max_concurrent, 4);
        assert_eq!(c.default_preset, Preset::Mp4Hevc);
        assert_eq!(c.default_subtitles, vec!["ko"]);
        assert_eq!(c.download_dir, PathBuf::from(&s.download.dir));
        s.download.preset = "bogus".into();
        s.download.max_concurrent = 0;
        let c = engine_config(&s);
        assert_eq!((c.default_preset, c.max_concurrent), (Preset::Original, 1));
    }
}
