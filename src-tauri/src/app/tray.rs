//! The menu-bar tray icon and menu, and the macOS app menu.

use crate::{
    app::{
        i18n::{self, Labels},
        updater, windows,
    },
    domain::settings::Settings,
};
use kiri_core::{engine::Engine, model::Job};
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::{
    AppHandle, Manager, Wry,
    image::Image,
    menu::{MenuBuilder, MenuEvent, MenuItem, SubmenuBuilder, WINDOW_SUBMENU_ID},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

/// Tauri emits no DoubleClick on macOS, so two left clicks are matched here.
#[derive(Default)]
pub struct DoubleClick {
    last: Mutex<Option<Instant>>,
}

impl DoubleClick {
    const WINDOW: Duration = Duration::from_millis(400);

    pub fn click(&self, now: Instant) -> bool {
        let mut last = self.last.lock().unwrap();
        match *last {
            Some(prev) if now.duration_since(prev) <= Self::WINDOW => {
                *last = None;
                true
            }
            _ => {
                *last = Some(now);
                false
            }
        }
    }
}

pub fn summary(l: &Labels, jobs: &[Job]) -> String {
    let active: Vec<&Job> = jobs.iter().filter(|j| j.state.is_active()).collect();
    if active.is_empty() {
        return l.idle.to_string();
    }
    let avg = active.iter().map(|j| j.progress).sum::<f32>() / active.len() as f32;
    l.active
        .replace("{n}", &active.len().to_string())
        .replace("{p}", &((avg * 100.0).round() as u32).to_string())
}

pub struct TrayItems {
    pub summary: MenuItem<Wry>,
    pub open: MenuItem<Wry>,
    pub update: MenuItem<Wry>,
    pub quit: MenuItem<Wry>,
    /// Quit in the app menu (⌘Q)
    pub app_quit: MenuItem<Wry>,
}

fn update_label(l: &Labels, pending: Option<updater::UpdateInfo>) -> String {
    match pending {
        Some(info) => l.install_update.replace("{}", &info.version),
        None => l.check_update.to_string(),
    }
}

/// After every update check.
pub fn relabel_update(app: &AppHandle) {
    let Some(items) = app.try_state::<TrayItems>() else {
        return;
    };
    let _ = items
        .update
        .set_text(update_label(&i18n::current(app), updater::status(app)));
}

/// When settings are saved. A no-op until the tray exists.
pub fn relabel(app: &AppHandle, settings: &Settings) {
    let l = i18n::labels(i18n::resolve(&settings.general.ui_language));
    if let Some(w) = app.get_webview_window(windows::SETTINGS) {
        let _ = w.set_title(l.settings_title);
    }
    let Some(items) = app.try_state::<TrayItems>() else {
        return;
    };
    let _ = items.open.set_text(l.open);
    let _ = items
        .update
        .set_text(update_label(&l, updater::status(app)));
    let _ = items.quit.set_text(l.quit);
    let _ = items.app_quit.set_text(l.quit);
    if let Some(engine) = app.try_state::<Engine>() {
        let _ = items.summary.set_text(summary(&l, &engine.list()));
    }
}

/// When the queue changes. Called inside the engine's notify critical section, so it
/// never waits on the main thread: set_text from another thread blocks until the main
/// thread runs it, which deadlocks if a sync command there (stop_job, ...) waits on the
/// notify lock.
pub fn relabel_queue(app: &AppHandle, jobs: &[Job]) {
    if app.try_state::<TrayItems>().is_none() {
        return;
    }
    let text = summary(&i18n::current(app), jobs);
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(items) = handle.try_state::<TrayItems>() {
            let _ = items.summary.set_text(text);
        }
    });
}

/// Shared by the tray menu and the app menu (Builder::on_menu_event). The tray's own
/// on_menu_event also receives every menu event, so they are handled here only.
pub fn on_menu_event(app: &AppHandle, event: MenuEvent) {
    match event.id.as_ref() {
        "open" => {
            let _ = windows::show_main(app);
        }
        "update" => updater::on_tray_click(app.clone()),
        // Goes through ExitRequested, so the quit confirmation applies.
        "quit" => app.exit(0),
        _ => {}
    }
}

/// The default macOS menu, except Quit is our own item. The default Quit (`terminate:`)
/// skips ExitRequested and with it the quit confirmation.
fn app_menu(app: &AppHandle, quit: &MenuItem<Wry>) -> tauri::Result<tauri::menu::Menu<Wry>> {
    let app_sub = SubmenuBuilder::new(app, app.package_info().name.clone())
        .about(None)
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .item(quit)
        .build()?;
    let edit = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;
    let window = SubmenuBuilder::with_id(app, WINDOW_SUBMENU_ID, "Window")
        .minimize()
        .separator()
        .close_window()
        .build()?;
    MenuBuilder::new(app)
        .items(&[&app_sub, &edit, &window])
        .build()
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let l = i18n::current(app);
    let summary_item = MenuItem::with_id(app, "summary", l.idle, false, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", l.open, true, None::<&str>)?;
    let update = MenuItem::with_id(
        app,
        "update",
        update_label(&l, updater::status(app)),
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", l.quit, true, None::<&str>)?;
    let menu = MenuBuilder::new(app)
        .items(&[&summary_item])
        .separator()
        .items(&[&open, &update])
        .separator()
        .items(&[&quit])
        .build()?;

    let clicks = DoubleClick::default();
    TrayIconBuilder::with_id("main")
        .icon(Image::from_bytes(include_bytes!(
            "../../icons/tray-template.png"
        ))?)
        .icon_as_template(true)
        .tooltip("kiri")
        .menu(&menu)
        .show_menu_on_left_click(false) // left click is for double-click, the menu is on right click
        .on_tray_icon_event(move |tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if clicks.click(Instant::now()) {
                    let _ = windows::show_main(tray.app_handle());
                }
            }
        })
        .build(app)?;

    let app_quit = MenuItem::with_id(app, "quit", l.quit, true, Some("CmdOrCtrl+Q"))?;
    app.set_menu(app_menu(app, &app_quit)?)?;

    app.manage(TrayItems {
        summary: summary_item,
        open,
        update,
        quit,
        app_quit,
    });
    relabel_queue(app, &app.state::<Engine>().list());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kiri_core::model::{JobOptions, JobSource, JobState, Preset};

    fn job(state: JobState, progress: f32) -> Job {
        Job {
            id: 1,
            source: JobSource::Youtube { url: "u".into() },
            title: "t".into(),
            thumbnail: None,
            duration_secs: None,
            quality_label: "720p".into(),
            options: JobOptions {
                format_id: None,
                preset: Preset::Original,
                subtitles: vec![],
                auto_subtitles: false,
                max_height: None,
            },
            state,
            progress,
            speed: None,
            eta: None,
            output: None,
            work_dir: None,
            original: None,
            created_at: 0,
        }
    }

    #[test]
    fn double_click_within_window() {
        let d = DoubleClick::default();
        let t0 = Instant::now();
        assert!(!d.click(t0));
        assert!(d.click(t0 + Duration::from_millis(300)));
        assert!(!d.click(t0 + Duration::from_millis(400))); // reset after a match
        assert!(!d.click(t0 + Duration::from_millis(1000))); // 600 ms apart
        assert!(d.click(t0 + Duration::from_millis(1350)));
    }

    #[test]
    fn summary_text() {
        let l = i18n::labels(i18n::Lang::Ko);
        assert_eq!(
            summary(&l, &[job(JobState::Completed, 1.0)]),
            "대기 중인 작업 없음"
        );
        assert_eq!(
            summary(
                &l,
                &[
                    job(JobState::Downloading, 0.4),
                    job(JobState::Encoding, 0.6),
                    job(JobState::Queued, 0.0)
                ]
            ),
            "진행 중 2개 · 50%"
        );
    }

    #[test]
    fn update_label_follows_pending_state() {
        let l = i18n::labels(i18n::Lang::Ko);
        assert_eq!(update_label(&l, None), "업데이트 확인");
        let info = updater::UpdateInfo {
            version: "0.2.0".into(),
            notes: String::new(),
            ready: false,
            downloading: false,
        };
        assert_eq!(update_label(&l, Some(info)), "v0.2.0 설치");
    }
}
