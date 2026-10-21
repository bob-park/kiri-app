//! App shell: managed state, windows, tray and menus, updater, CLI link, quit flow and native i18n.

pub mod cli_install;
pub mod i18n;
pub mod ipc_server;
#[cfg(test)]
mod pubkey_check;
pub mod quit;
pub mod state;
pub mod tray;
pub mod updater;
pub mod windows;

use std::path::PathBuf;

/// `externalBin` sidecars sit next to the executable (Contents/MacOS, or target/debug in dev).
pub fn sidecar(name: &str) -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join(name)))
        .unwrap_or_else(|| PathBuf::from(name))
}
