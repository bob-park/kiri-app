//! Quit confirmation: with downloads running, quitting asks first.
use crate::app::{i18n, windows};
use kiri_core::engine::Engine;
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use tauri::{AppHandle, Manager, RunEvent};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

static CONFIRMED: AtomicBool = AtomicBool::new(false);
/// The confirmation dialog is open (so repeated ⌘Q never stacks dialogs).
static DIALOG_OPEN: AtomicBool = AtomicBool::new(false);

/// Later quit requests no longer ask (the dialog's "Quit", an update restart).
pub fn allow_exit() {
    CONFIRMED.store(true, Ordering::SeqCst);
}

pub fn needs_confirm(confirmed: bool, has_active: bool) -> bool {
    !confirmed && has_active
}

pub fn on_run_event(app: &AppHandle, event: RunEvent) {
    match event {
        RunEvent::ExitRequested { api, .. } => {
            let active = app.try_state::<Engine>().is_some_and(|e| e.has_active());
            if needs_confirm(CONFIRMED.load(Ordering::SeqCst), active) {
                api.prevent_exit();
                confirm(app);
            }
        }
        // `terminate:` (e.g. Quit from the Dock menu) cannot be prevented. Requeue jobs
        // and stop child processes.
        RunEvent::Exit => {
            if let Some(engine) = app.try_state::<Engine>() {
                engine.shutdown_for_exit(Duration::from_secs(2));
            }
        }
        // Dock icon click
        RunEvent::Reopen { .. } => {
            let _ = windows::show_main(app);
        }
        _ => {}
    }
}

fn confirm(app: &AppHandle) {
    if DIALOG_OPEN.swap(true, Ordering::SeqCst) {
        return;
    }
    let l = i18n::current(app);
    let handle = app.clone();
    app.dialog()
        .message(l.quit_message)
        .title(l.quit_title)
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom(
            l.quit_ok.into(),
            l.quit_cancel.into(),
        ))
        .show(move |ok| {
            DIALOG_OPEN.store(false, Ordering::SeqCst);
            if ok {
                allow_exit();
                handle.exit(0);
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confirm_only_when_active_and_not_confirmed() {
        assert!(needs_confirm(false, true));
        assert!(!needs_confirm(true, true));
        assert!(!needs_confirm(false, false));
    }
}
