//! 테스트 전용 도우미. 실제 yt-dlp/ffmpeg 대신 sh 스크립트를 실행 파일로 쓴다.
use crate::model::{Job, JobOptions, JobState, Preset, Tools};
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

/// `-o <dir>/%(title)s.%(ext)s` 를 읽어 그 폴더에 결과물과 자막을 만든다.
pub const FAKE_YTDLP_DOWNLOAD: &str = r#"out=""
while [ $# -gt 0 ]; do case "$1" in -o) out="$2"; shift;; esac; shift; done
dir=$(dirname "$out")
echo "KIRI|  50.0%|  1.00MiB/s|00:01"
: > "$dir/Fake Video.mp4"
: > "$dir/Fake Video.ko.srt"
echo "KIRI|100.0%|  1.00MiB/s|00:00""#;

/// 마지막 인자(출력 경로)에 파일을 만든다.
pub const FAKE_FFMPEG: &str = r#"for a; do last="$a"; done
mkdir -p "$(dirname "$last")"
echo "out_time_us=1000000"
echo "progress=end"
: > "$last""#;

/// VideoToolbox 인코더를 쓰면 실패한다.
pub const FAKE_FFMPEG_VT_FAILS: &str = r#"case "$*" in *videotoolbox*) echo "Error: cannot open videotoolbox encoder" >&2; exit 1;; esac
for a; do last="$a"; done
mkdir -p "$(dirname "$last")"
: > "$last""#;

pub fn tools(dir: &Path, ytdlp_body: &str, ffmpeg_body: &str) -> Tools {
    Tools {
        ytdlp: script(dir, "yt-dlp", ytdlp_body),
        deno: dir.join("deno"),
        ffmpeg: script(dir, "ffmpeg", ffmpeg_body),
    }
}

pub fn job(preset: Preset) -> Job {
    Job {
        id: 1,
        url: "https://youtu.be/abc123".into(),
        title: "Fake Video".into(),
        thumbnail: None,
        duration_secs: Some(2.0),
        quality_label: "1080p".into(),
        options: JobOptions {
            format_id: Some("299".into()),
            preset,
            subtitles: vec!["ko".into()],
            auto_subtitles: false,
        },
        state: JobState::Downloading,
        progress: 0.0,
        speed: None,
        eta: None,
        output: None,
        created_at: 0,
    }
}
