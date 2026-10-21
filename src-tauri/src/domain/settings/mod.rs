//! Settings domain: read, patch and import the app settings stored in SQLite.

pub mod commands;
mod model;
pub mod repository;

pub use model::Settings;

use crate::shared::error::{AppError, AppResult};
use kiri_core::{engine::EngineConfig, model::Preset};
use rusqlite::Connection;
use serde_json::Value;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

/// Top-level sections, one `settings` row each.
const SECTIONS: [&str; 3] = ["general", "download", "update"];

/// Deep-merges `patch` into `base`: objects recurse, anything else (arrays too) replaces.
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

/// `current` with `patch` deep-merged in. A wrong type is `InvalidInput`.
pub fn apply_patch(current: &Settings, patch: &Value) -> AppResult<Settings> {
    let mut v = to_json(current)?;
    merge(&mut v, patch);
    serde_json::from_value(v).map_err(|e| AppError::InvalidInput(e.to_string()))
}

fn to_json(s: &Settings) -> AppResult<Value> {
    serde_json::to_value(s).map_err(|e| AppError::Failed(e.to_string()))
}

fn validate(s: &Settings) -> AppResult<()> {
    if !model::MAX_CONCURRENT.contains(&s.download.max_concurrent) {
        return Err(AppError::InvalidInput(format!(
            "download.max_concurrent must be 1-4, got {}",
            s.download.max_concurrent
        )));
    }
    Ok(())
}

/// Writes the sections of `next` that differ from `prev` in one transaction.
fn save(conn: &Connection, prev: &Value, next: &Value) -> AppResult<()> {
    let tx = conn.unchecked_transaction()?;
    for key in SECTIONS {
        if prev.get(key) != next.get(key) {
            repository::put(&tx, key, &next[key])?;
        }
    }
    tx.commit()?;
    Ok(())
}

/// Reads the legacy JSON file like the pre-SQLite app did: unreadable or corrupt
/// means defaults.
fn load_json(path: &Path) -> Settings {
    match fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
            log::warn!("settings.json parse error, using defaults: {e}");
            Settings::default()
        }),
        Err(e) => {
            if e.kind() != io::ErrorKind::NotFound {
                log::warn!("settings.json read error, using defaults: {e}");
            }
            Settings::default()
        }
    }
}

/// Current settings; missing sections are defaults.
pub fn get(conn: &Connection) -> AppResult<Settings> {
    let stored = Value::Object(repository::get_all(conn)?);
    let mut s: Settings = serde_json::from_value(stored).unwrap_or_else(|e| {
        log::warn!("stored settings do not fit the model, using defaults: {e}");
        Settings::default()
    });
    s.normalize();
    Ok(s)
}

/// Deep-merges `patch`, validates, and writes the changed sections in one transaction.
/// Invalid input saves nothing.
pub fn patch(conn: &Connection, patch: &Value) -> AppResult<Settings> {
    let current = get(conn)?;
    let mut next = apply_patch(&current, patch)?;
    validate(&next)?;
    next.normalize();
    save(conn, &to_json(&current)?, &to_json(&next)?)?;
    Ok(next)
}

/// One-time import of the legacy `settings.json`: only when the table is empty and the
/// file exists. The file is renamed to `settings.json.migrated`. Returns whether it ran.
pub fn import_json(conn: &Connection, path: &Path) -> AppResult<bool> {
    if !repository::get_all(conn)?.is_empty() || !path.exists() {
        return Ok(false);
    }
    let mut s = load_json(path);
    s.normalize();
    save(conn, &Value::Null, &to_json(&s)?)?;
    fs::rename(path, path.with_extension("json.migrated"))?;
    Ok(true)
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

#[cfg(test)]
mod tests {
    use super::model::General;
    use super::*;
    use crate::shared::db::open_in_memory;
    use serde_json::json;

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
    fn defaults_when_empty() {
        assert_eq!(get(&open_in_memory()).unwrap(), Settings::default());
    }

    #[test]
    fn patch_merges_and_persists() {
        let conn = open_in_memory();
        let returned = patch(&conn, &json!({"general": {"theme": "dark"}})).unwrap();
        let s = get(&conn).unwrap();
        assert_eq!(s, returned);
        assert_eq!(s.general.theme, "dark");
        let mut expected = Settings::default();
        expected.general.theme = "dark".into();
        assert_eq!(s, expected);
        // only the changed section is written
        let keys: Vec<String> = repository::get_all(&conn)
            .unwrap()
            .keys()
            .cloned()
            .collect();
        assert_eq!(keys, ["general"]);
    }

    #[test]
    fn stored_keys_are_snake_case() {
        let conn = open_in_memory();
        patch(
            &conn,
            &json!({"general": {"close_to_tray": false}, "download": {"max_concurrent": 3}}),
        )
        .unwrap();
        let all = repository::get_all(&conn).unwrap();
        assert_eq!(all["general"]["close_to_tray"], json!(false));
        assert_eq!(all["download"]["max_concurrent"], json!(3));
        let s = get(&conn).unwrap();
        assert!(!s.general.close_to_tray);
        assert_eq!(s.download.max_concurrent, 3);
    }

    #[test]
    fn ja_reads_as_system() {
        let conn = open_in_memory();
        repository::put(
            &conn,
            "general",
            &json!({"ui_language": "ja", "theme": "dark"}),
        )
        .unwrap();
        let s = get(&conn).unwrap();
        assert_eq!(s.general.ui_language, "system");
        assert_eq!(s.general.theme, "dark");
        let s = patch(&conn, &json!({"general": {"ui_language": "ja"}})).unwrap();
        assert_eq!(s.general.ui_language, "system");
        let s = patch(&conn, &json!({"general": {"ui_language": "ko"}})).unwrap();
        assert_eq!(s.general.ui_language, "ko");
    }

    #[test]
    fn invalid_patch_saves_nothing() {
        let conn = open_in_memory();
        patch(&conn, &json!({"general": {"theme": "dark"}})).unwrap();
        let before = get(&conn).unwrap();
        for bad in [
            json!({"download": {"max_concurrent": 0}, "general": {"theme": "light"}}),
            json!({"download": {"max_concurrent": 5}}),
            json!({"download": {"max_concurrent": "x"}}),
        ] {
            let err = patch(&conn, &bad).unwrap_err();
            assert!(matches!(err, AppError::InvalidInput(_)), "{bad}: {err:?}");
            assert_eq!(get(&conn).unwrap(), before, "{bad}");
        }
    }

    #[test]
    fn corrupt_row_falls_back_to_defaults() {
        let conn = open_in_memory();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES ('general', '{nope')",
            [],
        )
        .unwrap();
        repository::put(&conn, "download", &json!({"max_concurrent": 3})).unwrap();
        let s = get(&conn).unwrap();
        assert_eq!(s.general, General::default());
        assert_eq!(s.download.max_concurrent, 3);
    }

    #[test]
    fn import_json_moves_file() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("settings.json");
        fs::write(&path, r#"{"general":{"theme":"dark","ui_language":"ja"}}"#).unwrap();
        let conn = open_in_memory();
        assert!(import_json(&conn, &path).unwrap());
        let s = get(&conn).unwrap();
        assert_eq!(s.general.theme, "dark");
        assert_eq!(s.general.ui_language, "system");
        assert_eq!(s.download.max_concurrent, 2);
        assert!(!path.exists());
        assert!(d.path().join("settings.json.migrated").exists());
        assert!(!import_json(&conn, &path).unwrap());
    }

    #[test]
    fn import_json_skips_when_db_has_settings_or_file_missing() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("settings.json");
        let conn = open_in_memory();
        assert!(!import_json(&conn, &path).unwrap());

        patch(&conn, &json!({"general": {"theme": "light"}})).unwrap();
        fs::write(&path, r#"{"general":{"theme":"dark"}}"#).unwrap();
        assert!(!import_json(&conn, &path).unwrap());
        assert_eq!(get(&conn).unwrap().general.theme, "light");
        assert!(path.exists());
    }

    #[test]
    fn import_corrupt_json_saves_defaults() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("settings.json");
        fs::write(&path, "{nope").unwrap();
        let conn = open_in_memory();
        assert!(import_json(&conn, &path).unwrap());
        assert_eq!(get(&conn).unwrap(), Settings::default());
        assert!(d.path().join("settings.json.migrated").exists());
    }

    #[test]
    fn merge_recurses_and_replaces() {
        let mut base = json!({"general": {"theme": "system", "ui_language": "system"}, "download": {"subtitles": ["ko"]}});
        merge(
            &mut base,
            &json!({"general": {"theme": "dark"}, "download": {"subtitles": []}}),
        );
        assert_eq!(base["general"]["theme"], "dark");
        assert_eq!(base["general"]["ui_language"], "system");
        assert_eq!(base["download"]["subtitles"], json!([]));
    }

    #[test]
    fn apply_patch_merges_and_rejects_bad_types() {
        let mut cur = Settings::default();
        cur.update.last_ytdlp_check = Some(7);
        let next = apply_patch(
            &cur,
            &json!({"download": {"max_concurrent": 3}, "general": {"theme": "dark"}}),
        )
        .unwrap();
        assert_eq!(next.download.max_concurrent, 3);
        assert_eq!(next.general.theme, "dark");
        assert_eq!(next.update.last_ytdlp_check, Some(7));
        assert_eq!(next.download.dir, cur.download.dir);
        assert!(matches!(
            apply_patch(&cur, &json!({"download": {"max_concurrent": "x"}})),
            Err(AppError::InvalidInput(_))
        ));
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
