use crate::settings::{self, Settings, SettingsState};
use serde::Serialize;
use tauri::{AppHandle, State};

/// 프론트로 가는 오류. code 는 i18n 키 error.<code>.
#[derive(Serialize, Debug)]
pub struct CmdError {
    pub code: String,
    pub message: String,
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
    patch: serde_json::Value,
) -> CmdResult<()> {
    let mut merged = serde_json::to_value(state.get()).map_err(|e| e.to_string())?;
    settings::merge(&mut merged, &patch);
    let next: Settings = serde_json::from_value(merged).map_err(|e| e.to_string())?;
    state.set(&app, next)?;
    Ok(())
}
