//! Tauri adapters for the queue domain. The engine owns the queue; these just call it.

use super::{Job, NewJob};
use crate::shared::error::AppResult;
use kiri_core::{engine::Engine, ytdlp::VideoInfo};
use tauri::State;

/// Payload: the full job list. Emitted by the engine listener in `lib.rs`.
pub const QUEUE_CHANGED: &str = "queue://changed";

#[tauri::command]
pub fn list_jobs(engine: State<'_, Engine>) -> AppResult<Vec<Job>> {
    Ok(engine.list())
}

#[tauri::command]
pub async fn probe(engine: State<'_, Engine>, url: String) -> AppResult<VideoInfo> {
    Ok(engine.probe(&url).await?)
}

#[tauri::command]
pub fn add_job(engine: State<'_, Engine>, job: NewJob) -> AppResult<Job> {
    Ok(engine.add(job)?)
}

/// Adds with the settings defaults, skipping the sheet.
#[tauri::command]
pub async fn add_url(engine: State<'_, Engine>, url: String) -> AppResult<Job> {
    Ok(engine.add_url(&url, None, None, None).await?)
}

#[tauri::command]
pub fn stop_job(engine: State<'_, Engine>, id: u64) -> AppResult<()> {
    Ok(engine.stop(id)?)
}

#[tauri::command]
pub fn remove_job(engine: State<'_, Engine>, id: u64) -> AppResult<()> {
    Ok(engine.remove(id)?)
}

#[tauri::command]
pub fn restart_job(engine: State<'_, Engine>, id: u64) -> AppResult<()> {
    Ok(engine.restart(id)?)
}

#[tauri::command]
pub fn clear_completed(engine: State<'_, Engine>) -> AppResult<usize> {
    Ok(engine.clear_completed())
}
