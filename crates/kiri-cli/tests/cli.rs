//! 실제 소켓 서버(앱 대신 kiri-core Engine)에 CLI 바이너리를 붙여 본다.
use kiri_core::{
    engine::{Engine, EngineConfig, EnginePaths},
    ipc,
    model::{Preset, Tools},
};
use std::{path::Path, process::Command};

fn idle_engine(dir: &Path) -> Engine {
    let tools = Tools {
        ytdlp: dir.join("none/yt-dlp"),
        deno: dir.join("none/deno"),
        ffmpeg: dir.join("none/ffmpeg"),
        ffprobe: dir.join("none/ffprobe"),
    };
    let paths = EnginePaths {
        queue_file: dir.join("q.json"),
        cache_dir: dir.join("c"),
        log_dir: dir.join("l"),
    };
    let cfg = EngineConfig {
        max_concurrent: 1,
        hw_accel: false,
        download_dir: dir.join("out"),
        default_quality: "best".into(),
        default_preset: Preset::Original,
        default_subtitles: vec![],
    };
    Engine::new(paths, tools, cfg, tokio::runtime::Handle::current(), |_| {})
}

async fn kiri(socket: &Path, args: &[&str]) -> (i32, String, String) {
    let socket = socket.to_path_buf();
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    tokio::task::spawn_blocking(move || {
        let out = Command::new(env!("CARGO_BIN_EXE_kiri"))
            .args(&args)
            .env("KIRI_SOCKET", &socket)
            .output()
            .unwrap();
        (
            out.status.code().unwrap(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    })
    .await
    .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn talks_to_running_app() {
    let d = tempfile::tempdir().unwrap();
    let socket = d.path().join("kiri.sock");
    let listener = ipc::bind(&socket).await.unwrap();
    tokio::spawn(ipc::serve(listener, idle_engine(d.path())));

    let (code, out, _) = kiri(&socket, &["list"]).await;
    assert_eq!((code, out.trim()), (0, "no jobs"));

    let (code, out, _) = kiri(&socket, &["--json", "status"]).await;
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v, serde_json::json!({"type": "jobs", "jobs": []}));

    let (code, _, err) = kiri(&socket, &["stop", "42"]).await;
    assert_eq!(code, 1);
    assert!(err.contains("job 42 not found"), "{err}");

    let (code, _, err) = kiri(&socket, &["add", "hello"]).await;
    assert_eq!(code, 1);
    assert!(err.contains("not a YouTube URL"), "{err}");
}

#[tokio::test(flavor = "multi_thread")]
async fn exit_code_2_when_app_not_running() {
    let d = tempfile::tempdir().unwrap();
    let (code, _, err) = kiri(&d.path().join("none.sock"), &["list"]).await;
    assert_eq!(code, 2);
    assert!(err.contains("not running"), "{err}");
}

fn engine_with_ffprobe(dir: &Path) -> Engine {
    use std::os::unix::fs::PermissionsExt;
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let ffprobe = bin.join("ffprobe");
    std::fs::write(&ffprobe, "#!/bin/sh\necho 3.5\n").unwrap();
    std::fs::set_permissions(&ffprobe, std::fs::Permissions::from_mode(0o755)).unwrap();
    let tools = Tools {
        ytdlp: dir.join("none/yt-dlp"),
        deno: dir.join("none/deno"),
        ffmpeg: dir.join("none/ffmpeg"),
        ffprobe,
    };
    let paths = EnginePaths {
        queue_file: dir.join("q.json"),
        cache_dir: dir.join("c"),
        log_dir: dir.join("l"),
    };
    let cfg = EngineConfig {
        max_concurrent: 1,
        hw_accel: false,
        download_dir: dir.join("out"),
        default_quality: "best".into(),
        default_preset: Preset::Original,
        default_subtitles: vec![],
    };
    Engine::new(paths, tools, cfg, tokio::runtime::Handle::current(), |_| {})
}

async fn kiri_in(cwd: &Path, socket: &Path, args: &[&str]) -> (i32, String, String) {
    let (cwd, socket) = (cwd.to_path_buf(), socket.to_path_buf());
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    tokio::task::spawn_blocking(move || {
        let out = Command::new(env!("CARGO_BIN_EXE_kiri"))
            .args(&args)
            .current_dir(&cwd)
            .env("KIRI_SOCKET", &socket)
            .output()
            .unwrap();
        (
            out.status.code().unwrap(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    })
    .await
    .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn transcode_sends_absolute_path_with_spaces() {
    let d = tempfile::tempdir().unwrap();
    let socket = d.path().join("kiri.sock");
    let listener = ipc::bind(&socket).await.unwrap();
    tokio::spawn(ipc::serve(listener, engine_with_ffprobe(d.path())));
    std::fs::write(d.path().join("내 영상 01.mkv"), "x").unwrap();

    let (code, out, err) = kiri_in(
        d.path(),
        &socket,
        &[
            "--json",
            "transcode",
            "./내 영상 01.mkv",
            "--format",
            "mp4-hevc",
            "--quality",
            "720p",
        ],
    )
    .await;
    assert_eq!(code, 0, "{err}");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let real = std::fs::canonicalize(d.path()).unwrap();
    assert_eq!(v["type"], "added");
    assert_eq!(
        v["job"]["source"],
        serde_json::json!({"kind": "file", "path": real.join("내 영상 01.mkv"), "output_dir": null})
    );
    assert_eq!(v["job"]["title"], "내 영상 01.mkv");
    assert_eq!(v["job"]["quality_label"], "720p");

    let (code, out, _) = kiri_in(
        d.path(),
        &socket,
        &["transcode", "./내 영상 01.mkv", "--format", "mp3"],
    )
    .await;
    assert_eq!(code, 0);
    assert!(out.starts_with("added #2: 내 영상 01.mkv"), "{out}");
}

#[tokio::test(flavor = "multi_thread")]
async fn transcode_missing_source_exits_1_without_app() {
    let d = tempfile::tempdir().unwrap();
    // 앱이 없어도(소켓 없음) 원본 확인이 먼저라 2가 아니라 1로 끝난다.
    let (code, _, err) = kiri(
        &d.path().join("none.sock"),
        &["transcode", "/nope/x.mkv", "--format", "mp3"],
    )
    .await;
    assert_eq!(code, 1);
    assert!(err.contains("source not found"), "{err}");
}
