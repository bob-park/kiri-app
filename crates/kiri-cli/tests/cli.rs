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
