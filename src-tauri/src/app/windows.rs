//! The `main` and `settings` windows, Dock visibility and the `open_settings` command.

use crate::{
    app::{i18n, state::AppState},
    shared::error::{AppError, AppResult},
};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};

pub const MAIN: &str = "main";
pub const SETTINGS: &str = "settings";

/// No payload. Tells the settings page to go back to its first tab.
pub const SETTINGS_OPENED: &str = "settings://opened";

/// With every window hidden the app leaves the Dock and lives in the menu bar only.
pub fn set_dock_visible(app: &AppHandle, visible: bool) {
    let policy = if visible {
        tauri::ActivationPolicy::Regular
    } else {
        tauri::ActivationPolicy::Accessory
    };
    let _ = app.set_activation_policy(policy);
}

pub fn show_main(app: &AppHandle) -> Result<(), String> {
    set_dock_visible(app, true);
    if let Some(w) = app.get_webview_window(MAIN) {
        let _ = w.unminimize();
        w.show().map_err(|e| e.to_string())?;
        return w.set_focus().map_err(|e| e.to_string());
    }
    // Tauri's native drop handler stays on (no `disable_drag_drop_handler`): it swallows drops,
    // so a link dropped on the window can't navigate the webview. Input is ⌘V only.
    let w = WebviewWindowBuilder::new(app, MAIN, WebviewUrl::App("/".into()))
        .title("kiri")
        .inner_size(760.0, 680.0)
        .min_inner_size(600.0, 560.0)
        .build()
        .map_err(|e| e.to_string())?;
    let handle = w.clone();
    let app_handle = app.clone();
    w.on_window_event(move |e| {
        if let WindowEvent::CloseRequested { api, .. } = e {
            // The window is never destroyed. Close = hide, or quit the app (with confirmation).
            api.prevent_close();
            if app_handle
                .state::<AppState>()
                .settings()
                .general
                .close_to_tray
            {
                let _ = handle.hide();
                set_dock_visible(&app_handle, false);
            } else {
                app_handle.exit(0);
            }
        }
    });
    Ok(())
}

pub fn show_settings(app: &AppHandle) -> Result<(), String> {
    set_dock_visible(app, true);
    if let Some(w) = app.get_webview_window(SETTINGS) {
        w.show().map_err(|e| e.to_string())?;
        let _ = app.emit_to(SETTINGS, SETTINGS_OPENED, ());
        return w.set_focus().map_err(|e| e.to_string());
    }
    WebviewWindowBuilder::new(app, SETTINGS, WebviewUrl::App("/".into()))
        .title(i18n::current(app).settings_title)
        .inner_size(560.0, 460.0)
        .resizable(false)
        .build()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_settings(app: AppHandle) -> AppResult<()> {
    show_settings(&app).map_err(AppError::Failed)
}
