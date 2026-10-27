//! Test-only helpers. sh scripts stand in for the real yt-dlp/ffmpeg executables.
use crate::model::{Job, JobOptions, JobSource, JobState, Preset, Tools};
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

/// Reads `-o <dir>/%(title)s.%(ext)s` and creates the output and subtitles in that folder.
pub const FAKE_YTDLP_DOWNLOAD: &str = r#"out=""
while [ $# -gt 0 ]; do case "$1" in -o) out="$2"; shift;; esac; shift; done
dir=$(dirname "$out")
echo "[download] Destination: $dir/Fake Video.mp4"
echo "KIRI|  50.0%|1048576|00:01"
: > "$dir/Fake Video.mp4"
: > "$dir/Fake Video.ko.srt"
echo "KIRI|100.0%|1048576|00:00""#;

/// Creates a file at the last argument (the output path).
pub const FAKE_FFMPEG: &str = r#"for a; do last="$a"; done
mkdir -p "$(dirname "$last")"
echo "out_time_us=1000000"
echo "progress=end"
: > "$last""#;

/// Fails when a VideoToolbox encoder is used.
pub const FAKE_FFMPEG_VT_FAILS: &str = r#"case "$*" in *videotoolbox*) echo "Error: cannot open videotoolbox encoder" >&2; exit 1;; esac
for a; do last="$a"; done
mkdir -p "$(dirname "$last")"
: > "$last""#;

pub fn tools(dir: &Path, ytdlp_body: &str, ffmpeg_body: &str) -> Tools {
    Tools {
        ytdlp: script(dir, "yt-dlp", ytdlp_body),
        deno: script(dir, "deno", "echo deno 2.0"),
        ffmpeg: script(dir, "ffmpeg", ffmpeg_body),
        ffprobe: script(dir, "ffprobe", "echo 2.0"),
    }
}

pub fn job(preset: Preset) -> Job {
    Job {
        id: 1,
        source: JobSource::Youtube {
            url: "https://youtu.be/abc123".into(),
        },
        title: "Fake Video".into(),
        thumbnail: None,
        duration_secs: Some(2.0),
        quality_label: "1080p".into(),
        options: JobOptions {
            format_id: Some("299".into()),
            preset,
            subtitles: vec!["ko".into()],
            auto_subtitles: false,
            max_height: None,
        },
        state: JobState::Downloading,
        progress: 0.0,
        speed: None,
        eta: None,
        output: None,
        work_dir: None,
        original: None,
        created_at: 0,
    }
}

pub const PROBE_FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/probe.json");

/// With `-J` prints the fixture, otherwise runs download_body.
pub fn ytdlp_with_probe(download_body: &str) -> String {
    format!("case \" $* \" in *\" -J \"*) cat '{PROBE_FIXTURE}'; exit 0;; esac\n{download_body}")
}
