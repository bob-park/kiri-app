//! CLI 소켓 서버. 실패해도 앱은 계속 동작하고, 이유는 CLI 탭에 보여준다.
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
                eprintln!("kiri ipc: {e}");
                *app.state::<IpcState>().0.lock().unwrap() = Some(e.to_string());
            }
        }
    });
}
