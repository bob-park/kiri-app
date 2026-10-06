use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub const MAIN: &str = "main";

pub fn show_main(app: &AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window(MAIN) {
        w.show().map_err(|e| e.to_string())?;
        return w.set_focus().map_err(|e| e.to_string());
    }
    WebviewWindowBuilder::new(app, MAIN, WebviewUrl::App("/".into()))
        .title("kiri")
        .inner_size(720.0, 520.0)
        .min_inner_size(560.0, 400.0)
        // 링크 드롭을 HTML5 drop 이벤트로 받기 위해 Tauri 의 파일 드롭 처리를 끈다.
        .disable_drag_drop_handler()
        .build()
        .map(|_| ())
        .map_err(|e| e.to_string())
}
