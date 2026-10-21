//! Tauri adapters for the queue domain. The engine owns the queue; these just call it.

use super::{Job, NewJob};
use crate::{
    app::{tray, updater},
    shared::error::AppResult,
};
use kiri_core::{engine::Engine, ytdlp::VideoInfo};
use tauri::{AppHandle, Emitter, State};

/// Payload: the full job list. Emitted by `on_change`.
pub const QUEUE_CHANGED: &str = "queue://changed";

/// The engine's change listener, called serially. Calling an Engine mutator here
/// deadlocks, so this only emits and does tray work that never waits on the main thread.
pub fn on_change(app: &AppHandle, jobs: &[Job]) {
    let _ = app.emit(QUEUE_CHANGED, jobs);
    tray::relabel_queue(app, jobs);
    updater::on_queue_change(app, jobs);
}

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
