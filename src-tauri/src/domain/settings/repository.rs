//! SQL for the `settings` table: one row per top-level section, value is its JSON.

use crate::shared::error::AppResult;
use rusqlite::Connection;
use serde_json::{Map, Value};

/// Every stored section keyed by name. A row whose JSON no longer parses is skipped
/// (logged), so that section falls back to defaults.
pub fn get_all(conn: &Connection) -> AppResult<Map<String, Value>> {
    let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    let mut out = Map::new();
    for row in rows {
        let (key, text) = row?;
        match serde_json::from_str(&text) {
            Ok(v) => {
                out.insert(key, v);
            }
            Err(e) => log::warn!("settings row {key} is not valid JSON, using defaults: {e}"),
        }
    }
    Ok(out)
}

pub fn put(conn: &Connection, key: &str, value: &Value) -> AppResult<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        (key, value.to_string()),
    )?;
    Ok(())
}
