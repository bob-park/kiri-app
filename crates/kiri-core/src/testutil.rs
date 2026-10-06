//! 테스트 전용 도우미. 실제 yt-dlp/ffmpeg 대신 sh 스크립트를 실행 파일로 쓴다.
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

pub fn script(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}
