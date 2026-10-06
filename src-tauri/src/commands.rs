use crate::{
    bootstrap::{self, ToolsState, ToolsStatus},
    settings::{self, Settings, SettingsState},
};
use kiri_core::{
    engine::{Engine, EngineError},
    model::{Job, NewJob},
    ytdlp::VideoInfo,
};
use serde::Serialize;
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
            code: "unknown".into(),
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
pub fn add_job(engine: State<'_, Engine>, job: NewJob) -> Job {
    engine.add(job)
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
