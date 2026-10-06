//! 종료 확인. 다운로드가 돌고 있으면 바로 끄지 않고 묻는다.
use crate::{i18n, windows};
use kiri_core::engine::Engine;
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use tauri::{AppHandle, Manager, RunEvent};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

static CONFIRMED: AtomicBool = AtomicBool::new(false);
/// 확인 대화상자가 떠 있다(⌘Q 연타로 겹쳐 뜨지 않게).
static DIALOG_OPEN: AtomicBool = AtomicBool::new(false);

/// 이후의 종료 요청은 묻지 않는다(확인 대화상자의 "종료", 업데이트 재시작).
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
        // Dock 메뉴의 종료 등 `terminate:` 는 막을 수 없다. 작업을 대기로 되돌리고 자식을 정리한다.
        RunEvent::Exit => {
            if let Some(engine) = app.try_state::<Engine>() {
                engine.shutdown_for_exit(Duration::from_secs(2));
            }
        }
        // Dock 아이콘 클릭
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
