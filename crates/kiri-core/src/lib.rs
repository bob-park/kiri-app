//! kiri's Tauri-free core, shared by the app and the CLI.
pub mod engine;
pub mod ffmpeg;
pub mod files;
pub mod ipc;
pub mod model;
pub mod pipeline;
pub mod queue;
pub mod runner;
#[cfg(test)]
pub(crate) mod testutil;
pub mod tools;
pub mod ytdlp;
