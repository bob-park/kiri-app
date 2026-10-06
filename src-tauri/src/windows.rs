use crate::settings::SettingsState;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};

pub const MAIN: &str = "main";
pub const SETTINGS: &str = "settings";

/// 창을 숨기면 Dock 에서도 빠지고 메뉴 막대에만 남는다.
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
    let w = WebviewWindowBuilder::new(app, MAIN, WebviewUrl::App("/".into()))
        .title("kiri")
        .inner_size(720.0, 520.0)
        .min_inner_size(560.0, 400.0)
        // 링크 드롭을 HTML5 drop 이벤트로 받기 위해 Tauri 의 파일 드롭 처리를 끈다.
        .disable_drag_drop_handler()
        .build()
        .map_err(|e| e.to_string())?;
    let handle = w.clone();
    let app_handle = app.clone();
    w.on_window_event(move |e| {
        if let WindowEvent::CloseRequested { api, .. } = e {
            // 창은 항상 살려 둔다. 닫기 = 숨기기 또는 앱 종료(확인 포함).
            api.prevent_close();
            if app_handle
                .state::<SettingsState>()
                .get()
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
    if let Some(w) = app.get_webview_window(SETTINGS) {
        w.show().map_err(|e| e.to_string())?;
        return w.set_focus().map_err(|e| e.to_string());
    }
    WebviewWindowBuilder::new(app, SETTINGS, WebviewUrl::App("/".into()))
        .title(crate::i18n::current(app).settings_title)
        .inner_size(560.0, 460.0)
        .resizable(false)
        .build()
        .map(|_| ())
        .map_err(|e| e.to_string())
}
