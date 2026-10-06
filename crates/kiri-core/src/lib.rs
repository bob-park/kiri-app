//! kiri 의 Tauri 비의존 코어. 앱과 CLI 가 함께 쓴다.
pub mod ffmpeg;
pub mod files;
pub mod model;
pub mod pipeline;
pub mod queue;
pub mod runner;
#[cfg(test)]
pub(crate) mod testutil;
pub mod ytdlp;
