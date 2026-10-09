use crate::{
    bootstrap::{self, ToolsState, ToolsStatus},
    cli_install::{self, CliStatus},
    settings::{self, Settings, SettingsState},
};
use kiri_core::{
    engine::{Engine, EngineError},
    model::{Job, NewJob},
    ytdlp::VideoInfo,
};
use serde::Serialize;
use std::path::Path;
use tauri::{AppHandle, State};

/// 프론트로 가는 오류. code 는 i18n 키 error.<code>.
#[derive(Serialize, Debug)]
pub struct CmdError {
    pub code: String,
    pub message: String,
}

impl From<EngineError> for CmdError {
    fn from(e: EngineError) -> Self {
        Self {
            code: e.code().into(),
            message: e.to_string(),
        }
    }
}

impl From<String> for CmdError {
    fn from(message: String) -> Self {
        Self {
            code: "failed".into(),
            message,
        }
    }
}

pub type CmdResult<T> = Result<T, CmdError>;

#[tauri::command]
pub fn get_settings(state: State<'_, SettingsState>) -> Settings {
    state.get()
}

/// 부분 갱신. 전체 문서를 쓰면 동시 쓰기가 서로를 덮는다.
#[tauri::command]
pub fn patch_settings(
    app: AppHandle,
    state: State<'_, SettingsState>,
    engine: State<'_, Engine>,
    patch: serde_json::Value,
) -> CmdResult<()> {
    let next = state.update(&app, |s| {
        *s = settings::apply_patch(s, &patch)?;
        Ok(())
    })?;
    engine.set_config(settings::engine_config(&next));
    Ok(())
}

#[tauri::command]
pub fn list_jobs(engine: State<'_, Engine>) -> Vec<Job> {
    engine.list()
}

#[tauri::command]
pub async fn probe(engine: State<'_, Engine>, url: String) -> CmdResult<VideoInfo> {
    Ok(engine.probe(&url).await?)
}

#[tauri::command]
pub fn add_job(engine: State<'_, Engine>, job: NewJob) -> CmdResult<Job> {
    Ok(engine.add(job)?)
}

/// 시트 없이 설정 기본값으로 바로 추가.
#[tauri::command]
pub async fn add_url(engine: State<'_, Engine>, url: String) -> CmdResult<Job> {
    Ok(engine.add_url(&url, None, None, None).await?)
}

#[tauri::command]
pub fn stop_job(engine: State<'_, Engine>, id: u64) -> CmdResult<()> {
    Ok(engine.stop(id)?)
}

#[tauri::command]
pub fn remove_job(engine: State<'_, Engine>, id: u64) -> CmdResult<()> {
    Ok(engine.remove(id)?)
}

#[tauri::command]
pub fn clear_completed(engine: State<'_, Engine>) -> usize {
    engine.clear_completed()
}

#[tauri::command]
pub fn restart_job(engine: State<'_, Engine>, id: u64) -> CmdResult<()> {
    Ok(engine.restart(id)?)
}

#[tauri::command]
pub fn tools_status(state: State<'_, ToolsState>) -> ToolsStatus {
    state.get()
}

#[tauri::command]
pub async fn update_tools(app: AppHandle) {
    bootstrap::ensure(&app, true).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use kiri_core::engine::EngineError;

    #[test]
    fn engine_errors_keep_code_and_message() {
        let e: CmdError = EngineError::NotFound(4).into();
        assert_eq!(
            (e.code.as_str(), e.message.as_str()),
            ("not_found", "job 4 not found")
        );
    }
}

#[tauri::command]
pub fn update_status(app: AppHandle) -> Option<crate::updater::UpdateInfo> {
    crate::updater::status(&app)
}

#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<Option<crate::updater::UpdateInfo>, String> {
    crate::updater::check(&app).await
}

#[tauri::command]
pub async fn install_update(app: AppHandle, after_queue: bool) -> Result<(), String> {
    crate::updater::request_install(&app, after_queue).await
}

#[tauri::command]
pub fn retry_update_download(app: AppHandle) {
    crate::updater::start_download(&app);
}

#[tauri::command]
pub fn open_settings(app: AppHandle) -> CmdResult<()> {
    Ok(crate::windows::show_settings(&app)?)
}

#[tauri::command]
pub fn cli_status(ipc: State<'_, crate::ipc_server::IpcState>) -> CliStatus {
    let target = crate::sidecar("kiri-cli");
    let link = Path::new(cli_install::LINK);
    CliStatus {
        installed: cli_install::is_installed(&target, link),
        link: cli_install::LINK.into(),
        target: target.display().to_string(),
        socket_error: ipc.0.lock().unwrap().clone(),
    }
}

/// osascript 암호 창이 떠 있는 동안 메인 스레드를 막지 않도록 블로킹 스레드에서 돈다.
async fn blocking(f: impl FnOnce() -> Result<(), String> + Send + 'static) -> CmdResult<()> {
    Ok(tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())??)
}

#[tauri::command]
pub async fn install_cli() -> CmdResult<()> {
    blocking(|| cli_install::install(&crate::sidecar("kiri-cli"), Path::new(cli_install::LINK)))
        .await
}

#[tauri::command]
pub async fn uninstall_cli() -> CmdResult<()> {
    blocking(|| cli_install::uninstall(&crate::sidecar("kiri-cli"), Path::new(cli_install::LINK)))
        .await
}
