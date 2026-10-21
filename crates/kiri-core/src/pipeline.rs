//! One job end to end: download → encode (if needed) → move to the destination.
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
    /// The job's own folder. In the app: `<save folder>/<title>.kiripart` (created with the `.kiri` marker)
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
    /// Set once, on the report emitted right after the download is saved to the save folder.
    pub original: Option<PathBuf>,
}

#[derive(Debug, PartialEq)]
pub enum PipelineError {
    Cancelled,
    /// Message for the user. Starting with "error." means an i18n key.
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
    // The original is saved to the save folder as soon as it is downloaded, so the user can play it
    // while it converts. A restart converts from it again instead of downloading.
    let original = match job.original.as_deref().filter(|p| p.exists()) {
        Some(p) => p.to_path_buf(),
        None => {
            let original = download(job, url, cfg, cancel, report).await?;
            report(Report {
                stage: Stage::Downloading,
                progress: 1.0,
                speed: None,
                eta: None,
                original: Some(original.clone()),
            });
            original
        }
    };
    // On restart drop the earlier partial output (ffmpeg cannot resume encoding).
    let _ = fs::remove_dir_all(cfg.work_dir.join("out"));
    let Some(encoded) = encode(job, cfg, cancel, &original, report).await? else {
        return Ok(original);
    };
    // The encoded file keeps the original's final stem, so `-n` follows any " (n)" the original got.
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

/// Downloads into the work folder, then moves the media and subtitles to the save folder.
async fn download(
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
        original: None,
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
                original: None,
            });
        }
    })
    .await
    .map_err(|e| fail(cfg, e))?;

    let downloaded = files::find_media(&cfg.work_dir)
        .map_err(io_fail)?
        .ok_or_else(|| PipelineError::Failed(ERR_NO_OUTPUT.into()))?;
    finalize(cfg, &downloaded)
}

/// Local conversion: the source is only read; the result goes to the output folder as `{source}-{n}.{ext}`.
async fn transcode_file(
    job: &Job,
    src: &Path,
    cfg: &PipelineCfg,
    cancel: &CancellationToken,
    report: &mut impl FnMut(Report),
) -> Result<PathBuf, PipelineError> {
    // On restart drop the earlier partial output (ffmpeg cannot resume encoding).
    let _ = fs::remove_dir_all(cfg.work_dir.join("out"));
    let encoded = encode(job, cfg, cancel, src, report)
        .await?
        .ok_or_else(|| PipelineError::Failed(ERR_NO_OUTPUT.into()))?;
    let ext = encoded
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("mp4");
    // The package is named `{stem}-{n}.kiripart`, so the output takes the same number, or the next if taken.
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
        original: None,
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
                original: None,
            });
        }
    })
    .await
}

/// Moves the output and subtitles to the destination, adding " (n)" on name clashes.
/// Subtitles follow the final media name, since players pair them by stem.
fn finalize(cfg: &PipelineCfg, media: &Path) -> Result<PathBuf, PipelineError> {
    let name = media
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("video");
    let dest = files::unique_path(&cfg.download_dir, name);
    files::move_file(media, &dest).map_err(io_fail)?;
    // Subtitles are `<source stem>.<lang>.srt`. Encoding keeps the stem, so take it from media.
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
        go_job(e, &job(preset)).await
    }

    async fn go_job(e: &Env, j: &Job) -> (Result<PathBuf, PipelineError>, Vec<Report>) {
        let mut reports = vec![];
        let r = run(j, &e.cfg, &CancellationToken::new(), |r| reports.push(r)).await;
        (r, reports)
    }

    fn reported_original(reports: &[Report]) -> Vec<PathBuf> {
        reports.iter().filter_map(|r| r.original.clone()).collect()
    }

    #[tokio::test]
    async fn original_is_in_download_dir_before_encoding_starts() {
        let e = env(FAKE_YTDLP_DOWNLOAD, FAKE_FFMPEG, false);
        let original = e.cfg.download_dir.join("Fake Video.mp4");
        // ffmpeg fails unless the saved original exists and is the input it was given.
        let body = format!(
            "DOWNLOAD_DIR='{}'\n\
             [ -f \"$DOWNLOAD_DIR/Fake Video.mp4\" ] || exit 1\n\
             case \"$*\" in *\"-i $DOWNLOAD_DIR/Fake Video.mp4 \"*) ;; *) exit 1;; esac\n{FAKE_FFMPEG}",
            e.cfg.download_dir.display()
        );
        testutil::script(e.cfg.tools.ffmpeg.parent().unwrap(), "ffmpeg", &body);
        let (r, reports) = go(&e, Preset::Mp4H264).await;
        assert_eq!(r.unwrap(), e.cfg.download_dir.join("Fake Video-1.mp4"));
        assert_eq!(reported_original(&reports), vec![original.clone()]);
        assert!(original.exists());
        assert!(e.cfg.download_dir.join("Fake Video.ko.srt").exists());
    }

    #[tokio::test]
    async fn encode_failure_keeps_saved_original() {
        let e = env(FAKE_YTDLP_DOWNLOAD, "echo 'Error: boom' >&2; exit 1", true);
        let (r, reports) = go(&e, Preset::Mp4H264).await;
        assert_eq!(r, Err(PipelineError::Failed("Error: boom".into())));
        let original = e.cfg.download_dir.join("Fake Video.mp4");
        assert!(
            original.exists(),
            "original must survive a failed conversion"
        );
        assert_eq!(reported_original(&reports), vec![original]);
    }

    #[tokio::test]
    async fn resume_with_saved_original_skips_download() {
        let e = env("exit 1", FAKE_FFMPEG, true);
        let original = e.cfg.download_dir.join("Fake Video.mp4");
        fs::create_dir_all(&e.cfg.download_dir).unwrap();
        fs::write(&original, "orig").unwrap();
        let mut j = job(Preset::Mp4H264);
        j.original = Some(original.clone());
        let (r, reports) = go_job(&e, &j).await;
        assert_eq!(r.unwrap(), e.cfg.download_dir.join("Fake Video-1.mp4"));
        assert_eq!(fs::read_to_string(&original).unwrap(), "orig");
        assert!(reports.iter().all(|r| r.stage == Stage::Encoding));
    }

    #[tokio::test]
    async fn missing_saved_original_redownloads() {
        let e = env(FAKE_YTDLP_DOWNLOAD, FAKE_FFMPEG, true);
        let mut j = job(Preset::Mp4H264);
        j.original = Some(e.cfg.download_dir.join("gone.mp4"));
        let (r, reports) = go_job(&e, &j).await;
        assert_eq!(r.unwrap(), e.cfg.download_dir.join("Fake Video-1.mp4"));
        let original = e.cfg.download_dir.join("Fake Video.mp4");
        assert!(original.exists());
        assert_eq!(reported_original(&reports), vec![original]);
    }

    #[tokio::test]
    async fn collision_names_variant_after_final_original() {
        let e = env(FAKE_YTDLP_DOWNLOAD, FAKE_FFMPEG, true);
        fs::create_dir_all(&e.cfg.download_dir).unwrap();
        fs::write(e.cfg.download_dir.join("Fake Video.mp4"), "old").unwrap();
        let (r, reports) = go(&e, Preset::MovProres).await;
        assert_eq!(r.unwrap(), e.cfg.download_dir.join("Fake Video (1)-1.mov"));
        let original = e.cfg.download_dir.join("Fake Video (1).mp4");
        assert!(original.exists());
        assert_eq!(reported_original(&reports), vec![original]);
        assert_eq!(
            fs::read_to_string(e.cfg.download_dir.join("Fake Video.mp4")).unwrap(),
            "old"
        );
    }

    #[tokio::test]
    async fn original_downloads_and_moves_with_subtitles() {
        let e = env(FAKE_YTDLP_DOWNLOAD, FAKE_FFMPEG, true);
        let (r, reports) = go(&e, Preset::Original).await;
        let out = r.unwrap();
        assert_eq!(out, e.cfg.download_dir.join("Fake Video.mp4"));
        assert!(out.exists());
        assert!(e.cfg.download_dir.join("Fake Video.ko.srt").exists());
        // video + audio job, so video at 50% is 47.5% overall (video is 95%)
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
        // the original has no clash, so Fake Video.mp4; the conversion is -2, after the existing -1
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
        assert!(!e.cfg.log_path.exists()); // no retry log expected
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
        // Subtitle names must follow the final media name (players pair by stem).
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
