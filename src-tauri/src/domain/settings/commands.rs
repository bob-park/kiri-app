//! Tauri adapters for the settings domain.

use super::Settings;
use crate::{app::state::AppState, shared::error::AppResult};
use kiri_core::engine::Engine;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager, State};

/// Payload: the new `Settings`. Every window listens to stay in sync.
pub const SETTINGS_CHANGED: &str = "settings://changed";

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> AppResult<Settings> {
    super::get(&state.db.lock().unwrap())
}

/// Partial update. Writing the whole document would let concurrent writers clobber each other.
#[tauri::command]
pub fn patch_settings(app: AppHandle, patch: Value) -> AppResult<Settings> {
    patch_and_notify(&app, &patch)
}

/// Applies `patch` and runs every side effect of a settings change: the event, the
/// engine config and the tray/menu labels (language). Shell code that changes a
/// setting (bootstrap's `last_ytdlp_check`) goes through here too.
pub fn patch_and_notify(app: &AppHandle, patch: &Value) -> AppResult<Settings> {
    // The guard is a temporary of this statement, so the DB lock is released before
    // the side effects below (tray relabel reads settings again).
    let next = super::patch(&app.state::<AppState>().db.lock().unwrap(), patch)?;
    let _ = app.emit(SETTINGS_CHANGED, &next);
    if let Some(engine) = app.try_state::<Engine>() {
        engine.set_config(super::engine_config(&next));
    }
    crate::tray::relabel(app, &next); // no-op until the tray exists
    Ok(next)
}
