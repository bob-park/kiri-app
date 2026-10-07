//! 작업 하나의 전 과정: 다운로드 → (필요하면) 인코딩 → 저장 위치로 이동.
use crate::{
    ffmpeg, files,
    model::{Job, JobSource, Tools},
    runner::{self, RunError},
    ytdlp,
};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use tokio_util::sync::CancellationToken;

pub const ERR_DIR_UNWRITABLE: &str = "error.download_dir_unwritable";
pub const ERR_NO_OUTPUT: &str = "error.no_output";

#[derive(Clone, Debug)]
pub struct PipelineCfg {
    pub tools: Tools,
    pub hw_accel: bool,
    pub download_dir: PathBuf,
    /// 작업 전용 폴더. 앱에서는 `<저장 폴더>/<제목>.kiripart` (표식 `.kiri` 를 함께 만든다)
    pub work_dir: PathBuf,
    pub log_path: PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Stage {
    Downloading,
    Encoding,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    pub stage: Stage,
    pub progress: f32,
    pub speed: Option<String>,
    pub eta: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum PipelineError {
    Cancelled,
    /// 사용자에게 보일 메시지. "error." 으로 시작하면 i18n 키.
    Failed(String),
}

fn append_log(cfg: &PipelineCfg, lines: &[String]) {
    if let Some(dir) = cfg.log_path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    if let Ok(mut f) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&cfg.log_path)
    {
        for l in lines {
            let _ = writeln!(f, "{l}");
        }
    }
}

fn fail(cfg: &PipelineCfg, e: RunError) -> PipelineError {
    match e {
        RunError::Cancelled => PipelineError::Cancelled,
        RunError::Failed { ref tail } => {
            append_log(cfg, tail);
            PipelineError::Failed(e.summary())
        }
        RunError::Spawn(ref msg) => {
            append_log(cfg, std::slice::from_ref(msg));
            PipelineError::Failed(e.summary())
        }
    }
}

fn io_fail(e: std::io::Error) -> PipelineError {
    PipelineError::Failed(e.to_string())
}

pub async fn run(
    job: &Job,
    cfg: &PipelineCfg,
    cancel: &CancellationToken,
    mut report: impl FnMut(Report),
) -> Result<PathBuf, PipelineError> {
    if files::check_writable(&cfg.download_dir).is_err() {
        return Err(PipelineError::Failed(ERR_DIR_UNWRITABLE.into()));
    }
    files::make_part_dir(&cfg.work_dir).map_err(io_fail)?;
    match &job.source {
        JobSource::Youtube { url } => download_and_save(job, url, cfg, cancel, &mut report).await,
        JobSource::File { path, .. } => transcode_file(job, path, cfg, cancel, &mut report).await,
    }
}

async fn download_and_save(
    job: &Job,
    url: &str,
    cfg: &PipelineCfg,
    cancel: &CancellationToken,
    report: &mut impl FnMut(Report),
) -> Result<PathBuf, PipelineError> {
    report(Report {
        stage: Stage::Downloading,
        progress: 0.0,
        speed: None,
        eta: None,
    });
    let args = ytdlp::download_args(url, &job.options, &cfg.tools, &cfg.work_dir);
    let mut tracker = ytdlp::ProgressTracker::new(ytdlp::expected_streams(&job.options));
    runner::run(&cfg.tools.ytdlp, &args, cancel, |line| {
        if let Some(p) = tracker.feed(line) {
            report(Report {
                stage: Stage::Downloading,
                progress: p.fraction,
                speed: p.speed,
                eta: p.eta,
            });
        }
    })
    .await
    .map_err(|e| fail(cfg, e))?;

    let downloaded = files::find_media(&cfg.work_dir)
        .map_err(io_fail)?
        .ok_or_else(|| PipelineError::Failed(ERR_NO_OUTPUT.into()))?;
    match encode(job, cfg, cancel, &downloaded, report).await? {
        None => finalize(cfg, &downloaded),
        Some(encoded) => {
            // 원본(과 자막)을 먼저 저장 폴더로 옮기고, 그 최종 이름을 기준으로 변환본 이름을 정한다.
            let original = finalize(cfg, &downloaded)?;
            let stem = original
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("video");
            let ext = encoded
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("mp4");
            let dest = files::variant_path(&cfg.download_dir, stem, ext);
            files::move_file(&encoded, &dest).map_err(io_fail)?;
            Ok(dest)
        }
    }
}

/// 로컬 파일 변환: 원본은 읽기만 하고, 결과를 `{원본}-{n}.{확장자}` 로 결과 폴더에 둔다.
async fn transcode_file(
    job: &Job,
    src: &Path,
    cfg: &PipelineCfg,
    cancel: &CancellationToken,
    report: &mut impl FnMut(Report),
) -> Result<PathBuf, PipelineError> {
    // 재시작이면 이전 부분 출력을 버린다(ffmpeg 는 이어서 인코딩하지 못한다).
    let _ = fs::remove_dir_all(cfg.work_dir.join("out"));
    let encoded = encode(job, cfg, cancel, src, report)
        .await?
        .ok_or_else(|| PipelineError::Failed(ERR_NO_OUTPUT.into()))?;
    let ext = encoded
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("mp4");
    // 패키지 이름이 `{stem}-{n}.kiripart` 이므로 같은 번호로 꺼낸다. 이미 있으면 다음 번호.
    let pkg_stem = cfg
        .work_dir
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let mut dest = cfg.download_dir.join(format!("{pkg_stem}.{ext}"));
    if dest.exists() {
        let stem = files::safe_title(src.file_stem().and_then(|s| s.to_str()).unwrap_or("file"));
        dest = files::variant_path(&cfg.download_dir, &stem, ext);
    }
    files::move_file(&encoded, &dest).map_err(io_fail)?;
    Ok(dest)
}

async fn encode(
    job: &Job,
    cfg: &PipelineCfg,
    cancel: &CancellationToken,
    input: &Path,
    report: &mut impl FnMut(Report),
) -> Result<Option<PathBuf>, PipelineError> {
    let preset = job.options.preset;
    let Some(ext) = preset.extension() else {
        return Ok(None);
    };
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let out_dir = cfg.work_dir.join("out");
    fs::create_dir_all(&out_dir).map_err(io_fail)?;
    let output = out_dir.join(format!("{stem}.{ext}"));
    let hw = cfg.hw_accel && ffmpeg::hw_capable(preset);

    report(Report {
        stage: Stage::Encoding,
        progress: 0.0,
        speed: None,
        eta: None,
    });
    match encode_once(job, cfg, cancel, input, &output, hw, report).await {
        Err(e @ RunError::Failed { .. }) if hw => {
            let mut lines =
                vec!["videotoolbox encode failed, retrying with software encoder:".to_string()];
            if let RunError::Failed { tail } = &e {
                lines.extend(tail.iter().cloned());
            }
            append_log(cfg, &lines);
            encode_once(job, cfg, cancel, input, &output, false, report)
                .await
                .map_err(|e| fail(cfg, e))?;
        }
        r => r.map_err(|e| fail(cfg, e))?,
    }
    Ok(Some(output))
}

async fn encode_once(
    job: &Job,
    cfg: &PipelineCfg,
    cancel: &CancellationToken,
    input: &Path,
    output: &Path,
    hw: bool,
    report: &mut impl FnMut(Report),
) -> Result<(), RunError> {
    let args = ffmpeg::encode_args(
        job.options.preset,
        hw,
        job.options.max_height,
        input,
        output,
    )
    .expect("preset with extension");
    let duration = job.duration_secs;
    runner::run(&cfg.tools.ffmpeg, &args, cancel, |line| {
        if let Some(p) = ffmpeg::parse_progress(line, duration) {
            report(Report {
                stage: Stage::Encoding,
                progress: p,
                speed: None,
                eta: None,
            });
        }
    })
    .await
}

/// 결과물과 자막을 저장 위치로 옮긴다. 같은 이름이 있으면 " (n)" 을 붙인다.
/// 자막은 플레이어가 파일 이름(stem)으로 짝을 찾으므로 최종 미디어 이름을 따라간다.
fn finalize(cfg: &PipelineCfg, media: &Path) -> Result<PathBuf, PipelineError> {
    let name = media
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("video");
    let dest = files::unique_path(&cfg.download_dir, name);
    files::move_file(media, &dest).map_err(io_fail)?;
    // 자막은 `<원본 stem>.<lang>.srt`. 인코딩 결과도 stem 은 그대로이므로 media 에서 얻는다.
    let stem = media
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    let dest_stem = dest.file_stem().and_then(|s| s.to_str()).unwrap_or(stem);
    for srt in files::subtitle_files(&cfg.work_dir).unwrap_or_default() {
        if let Some(n) = srt.file_name().and_then(|n| n.to_str()) {
            let renamed = n
                .strip_prefix(stem)
                .filter(|_| !stem.is_empty())
                .map_or_else(|| n.to_string(), |rest| format!("{dest_stem}{rest}"));
            let _ = files::move_file(&srt, &files::unique_path(&cfg.download_dir, &renamed));
        }
    }
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        model::Preset,
        testutil::{self, *},
    };
    use std::os::unix::fs::PermissionsExt;

    struct Env {
        _d: tempfile::TempDir,
        cfg: PipelineCfg,
    }

    fn env(ytdlp: &str, ffmpeg: &str, hw: bool) -> Env {
        let d = tempfile::tempdir().unwrap();
        let bin = d.path().join("bin");
        fs::create_dir(&bin).unwrap();
        let cfg = PipelineCfg {
            tools: testutil::tools(&bin, ytdlp, ffmpeg),
            hw_accel: hw,
            download_dir: d.path().join("Movies/kiri"),
            work_dir: d.path().join("Movies/kiri/Fake Video.kiripart"),
            log_path: d.path().join("logs/1.log"),
        };
        Env { _d: d, cfg }
    }

    async fn go(e: &Env, preset: Preset) -> (Result<PathBuf, PipelineError>, Vec<Report>) {
        let mut reports = vec![];
        let r = run(&job(preset), &e.cfg, &CancellationToken::new(), |r| {
            reports.push(r)
        })
        .await;
        (r, reports)
    }

    #[tokio::test]
    async fn original_downloads_and_moves_with_subtitles() {
        let e = env(FAKE_YTDLP_DOWNLOAD, FAKE_FFMPEG, true);
        let (r, reports) = go(&e, Preset::Original).await;
        let out = r.unwrap();
        assert_eq!(out, e.cfg.download_dir.join("Fake Video.mp4"));
        assert!(out.exists());
        assert!(e.cfg.download_dir.join("Fake Video.ko.srt").exists());
        // 영상+오디오 작업이라 영상 50% 는 전체의 47.5% (영상 몫 95%)
        assert!(
            reports
                .iter()
                .any(|r| r.stage == Stage::Downloading && (r.progress - 0.475).abs() < 1e-4)
        );
        assert!(reports.iter().all(|r| r.stage == Stage::Downloading));
        assert!(reports.windows(2).all(|w| w[1].progress >= w[0].progress));
    }

    #[tokio::test]
    async fn encodes_when_preset_needs_it() {
        let e = env(FAKE_YTDLP_DOWNLOAD, FAKE_FFMPEG, true);
        let (r, reports) = go(&e, Preset::MovProres).await;
        assert_eq!(r.unwrap(), e.cfg.download_dir.join("Fake Video-1.mov"));
        assert!(
            e.cfg.download_dir.join("Fake Video.mp4").exists(),
            "original kept"
        );
        assert!(e.cfg.download_dir.join("Fake Video.ko.srt").exists());
        assert!(
            reports
                .iter()
                .any(|r| r.stage == Stage::Encoding && r.progress == 0.5)
        );
    }

    #[tokio::test]
    async fn second_conversion_gets_next_index() {
        let e = env(FAKE_YTDLP_DOWNLOAD, FAKE_FFMPEG, true);
        fs::create_dir_all(&e.cfg.download_dir).unwrap();
        fs::write(e.cfg.download_dir.join("Fake Video-1.mov"), "earlier").unwrap();
        let (r, _) = go(&e, Preset::Mp4H264).await;
        // 원본은 같은 이름이 없으니 Fake Video.mp4, 변환본은 기존 -1 다음인 -2
        assert_eq!(r.unwrap(), e.cfg.download_dir.join("Fake Video-2.mp4"));
        assert!(e.cfg.download_dir.join("Fake Video.mp4").exists());
    }

    #[tokio::test]
    async fn audio_preset_keeps_downloaded_original() {
        let e = env(FAKE_YTDLP_DOWNLOAD, FAKE_FFMPEG, true);
        let (r, _) = go(&e, Preset::Mp3).await;
        assert_eq!(r.unwrap(), e.cfg.download_dir.join("Fake Video-1.mp3"));
        assert!(e.cfg.download_dir.join("Fake Video.mp4").exists());
    }

    #[tokio::test]
    async fn falls_back_to_software_when_videotoolbox_fails() {
        let e = env(FAKE_YTDLP_DOWNLOAD, FAKE_FFMPEG_VT_FAILS, true);
        let (r, _) = go(&e, Preset::Mp4H264).await;
        assert_eq!(r.unwrap(), e.cfg.download_dir.join("Fake Video-1.mp4"));
        let log = fs::read_to_string(&e.cfg.log_path).unwrap();
        assert!(log.contains("videotoolbox"), "{log}");
    }

    #[tokio::test]
    async fn hw_off_never_uses_videotoolbox() {
        let e = env(FAKE_YTDLP_DOWNLOAD, FAKE_FFMPEG_VT_FAILS, false);
        let (r, _) = go(&e, Preset::Mp4H264).await;
        assert!(r.is_ok());
        assert!(!e.cfg.log_path.exists()); // 재시도 로그가 없어야 한다
    }

    #[tokio::test]
    async fn download_failure_reports_last_line_and_logs_tail() {
        let e = env(
            "echo 'ERROR: [youtube] abc: Video unavailable' >&2; exit 1",
            FAKE_FFMPEG,
            true,
        );
        let (r, _) = go(&e, Preset::Original).await;
        assert_eq!(
            r,
            Err(PipelineError::Failed(
                "ERROR: [youtube] abc: Video unavailable".into()
            ))
        );
        assert!(
            fs::read_to_string(&e.cfg.log_path)
                .unwrap()
                .contains("Video unavailable")
        );
    }

    #[tokio::test]
    async fn unwritable_download_dir_fails_before_download() {
        let e = env("touch \"$0.ran\"", FAKE_FFMPEG, true);
        fs::create_dir_all(&e.cfg.download_dir).unwrap();
        fs::set_permissions(&e.cfg.download_dir, fs::Permissions::from_mode(0o555)).unwrap();
        let (r, _) = go(&e, Preset::Original).await;
        fs::set_permissions(&e.cfg.download_dir, fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(r, Err(PipelineError::Failed(ERR_DIR_UNWRITABLE.into())));
        let ran = e.cfg.tools.ytdlp.with_extension("ran");
        assert!(!ran.exists(), "yt-dlp must not run");
    }

    #[tokio::test]
    async fn existing_file_is_not_overwritten() {
        let e = env(FAKE_YTDLP_DOWNLOAD, FAKE_FFMPEG, true);
        fs::create_dir_all(&e.cfg.download_dir).unwrap();
        fs::write(e.cfg.download_dir.join("Fake Video.mp4"), "old").unwrap();
        let (r, _) = go(&e, Preset::Original).await;
        assert_eq!(r.unwrap(), e.cfg.download_dir.join("Fake Video (1).mp4"));
        assert_eq!(
            fs::read_to_string(e.cfg.download_dir.join("Fake Video.mp4")).unwrap(),
            "old"
        );
        // 자막 이름은 최종 미디어 이름을 따라가야 한다 (플레이어가 stem 으로 짝을 찾는다).
        assert!(e.cfg.download_dir.join("Fake Video (1).ko.srt").exists());
        assert!(!e.cfg.download_dir.join("Fake Video.ko.srt").exists());
    }

    #[tokio::test]
    async fn cancel_returns_cancelled() {
        let e = env("sleep 30", FAKE_FFMPEG, true);
        let token = CancellationToken::new();
        let t2 = token.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            t2.cancel();
        });
        let r = run(&job(Preset::Original), &e.cfg, &token, |_| {}).await;
        assert_eq!(r, Err(PipelineError::Cancelled));
    }
}
