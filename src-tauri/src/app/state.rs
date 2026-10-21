//! State managed by Tauri and shared by commands and shell code.

use crate::domain::settings::{self, Settings};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
}

impl AppState {
    /// Current settings for shell code that cannot surface an error (window close,
    /// tray labels, periodic checks). A read failure logs and yields defaults.
    pub fn settings(&self) -> Settings {
        settings::get(&self.db.lock().unwrap()).unwrap_or_else(|e| {
            log::error!("cannot read settings, using defaults: {e}");
            Settings::default()
        })
    }
}
