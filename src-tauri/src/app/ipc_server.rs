//! CLI socket server. A failure leaves the app running and is shown in the CLI tab.
use kiri_core::{engine::Engine, ipc};
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

#[derive(Default)]
pub struct IpcState(pub Mutex<Option<String>>);

pub fn spawn(app: &AppHandle, engine: Engine) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        match ipc::bind(&ipc::socket_path()).await {
            Ok(listener) => ipc::serve(listener, engine).await,
            Err(e) => {
                log::error!("cli socket unavailable: {e}");
                *app.state::<IpcState>().0.lock().unwrap() = Some(e.to_string());
            }
        }
    });
}
