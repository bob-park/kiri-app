//! One per app: ties queue state, concurrent runs and process cancellation together. The UI and the CLI share the same Engine.
use crate::{
    files,
    model::{Job, JobOptions, JobSource, JobState, NewJob, Preset, Tools},
    pipeline::{self, PipelineCfg, PipelineError, Report, Stage},
    queue::{QueueState, QueueStore},
    runner,
    ytdlp::{self, VideoInfo},
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::runtime::Handle;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug, PartialEq)]
pub struct EngineConfig {
    pub max_concurrent: usize,
    pub hw_accel: bool,
    pub download_dir: PathBuf,
    /// Default for CLI add ("best" | "audio" | "<N>p")
    pub default_quality: String,
    pub default_preset: Preset,
    pub default_subtitles: Vec<String>,
}

#[derive(Clone)]
pub struct EnginePaths {
    pub store: Arc<dyn QueueStore>,
    pub cache_dir: PathBuf,
    pub log_dir: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("job {0} not found")]
    NotFound(u64),
    #[error("job {0} is not running or queued")]
    NotStoppable(u64),
    #[error("job {0} cannot be restarted")]
    NotRestartable(u64),
    #[error("yt-dlp is not installed yet")]
    YtdlpMissing,
    #[error("not a YouTube URL")]
    InvalidUrl,
    #[error("{0}")]
    Probe(String),
    #[error("unknown quality: {0}")]
    BadQuality(String),
    #[error("unknown format: {0}")]
    BadPreset(String),
    #[error("source not found: {0}")]
    SourceNotFound(String),
    #[error("{0}")]
    InvalidMedia(String),
}

impl EngineError {
    /// Stable code used for the frontend i18n key (error.<code>) and CLI responses.
    pub fn code(&self) -> &'static str {
        match self {
            EngineError::NotFound(_) => "not_found",
            EngineError::NotStoppable(_) => "not_stoppable",
            EngineError::NotRestartable(_) => "not_restartable",
            EngineError::YtdlpMissing => "ytdlp_missing",
            EngineError::InvalidUrl => "invalid_url",
            EngineError::Probe(_) => "probe_failed",
            EngineError::BadQuality(_) => "bad_quality",
            EngineError::BadPreset(_) => "bad_preset",
            EngineError::SourceNotFound(_) => "source_not_found",
            EngineError::InvalidMedia(_) => "invalid_media",
        }
    }
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

type Listener = Box<dyn Fn(&[Job]) + Send + Sync>;

struct Inner {
    state: Mutex<QueueState>,
    running: Mutex<HashMap<u64, CancellationToken>>,
    config: Mutex<EngineConfig>,
    paths: EnginePaths,
    tools: Tools,
    rt: Handle,
    listener: Listener,
    /// Serializes building a snapshot together with calling the listener. Without it a
    /// later snapshot can go out first and leave the UI stuck on stale state.
    notify_lock: Mutex<()>,
    /// The app is quitting; no more jobs start.
    shutting_down: AtomicBool,
}

#[derive(Clone)]
pub struct Engine(Arc<Inner>);

impl Engine {
    /// `listener` is called with the latest list on every queue change. Calls are always serial
    /// and ordered. In exchange, the listener must not call Engine mutators (add/stop/…)
    /// synchronously (re-entrant deadlock).
    pub fn new(
        paths: EnginePaths,
        tools: Tools,
        config: EngineConfig,
        rt: Handle,
        listener: impl Fn(&[Job]) + Send + Sync + 'static,
    ) -> Engine {
        let (mut state, trusted) = paths.store.load();
        state.recover();
        // Job cache of older versions. Jobs in progress restart from scratch.
        let _ = std::fs::remove_dir_all(paths.cache_dir.join("jobs"));
        // Packages not in the queue (e.g. quit mid-removal) are never reused. Only marked ones are removed.
        // With a corrupt queue file we cannot tell which packages are in use, so none are touched.
        if trusted && let Ok(entries) = std::fs::read_dir(&config.download_dir) {
            let canon = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
            let listed: Vec<PathBuf> = state
                .jobs
                .iter()
                .filter_map(|j| j.work_dir.as_deref().map(canon))
                .collect();
            for e in entries.flatten() {
                let p = e.path();
                if !listed.contains(&canon(&p)) {
                    files::remove_part_dir(&p);
                }
            }
        }
        let engine = Engine(Arc::new(Inner {
            state: Mutex::new(state),
            running: Mutex::new(HashMap::new()),
            config: Mutex::new(config),
            paths,
            tools,
            rt,
            listener: Box::new(listener),
            notify_lock: Mutex::new(()),
            shutting_down: AtomicBool::new(false),
        }));
        engine.save();
        engine
    }

    /// Starts running queued jobs. Call once after setup (once the listener is ready).
    pub fn start(&self) {
        self.pump();
    }

    pub fn tools(&self) -> &Tools {
        &self.0.tools
    }

    pub fn list(&self) -> Vec<Job> {
        self.0.state.lock().unwrap().jobs.clone()
    }

    pub fn running(&self) -> Vec<Job> {
        self.list()
            .into_iter()
            .filter(|j| j.state.is_active())
            .collect()
    }

    pub fn has_active(&self) -> bool {
        self.0
            .state
            .lock()
            .unwrap()
            .jobs
            .iter()
            .any(|j| j.state.is_active())
    }

    /// No job is queued or running.
    pub fn is_idle(&self) -> bool {
        !self
            .0
            .state
            .lock()
            .unwrap()
            .jobs
            .iter()
            .any(|j| j.state.is_pending())
    }

    pub fn config(&self) -> EngineConfig {
        self.0.config.lock().unwrap().clone()
    }

    pub fn set_config(&self, c: EngineConfig) {
        *self.0.config.lock().unwrap() = c;
        self.pump(); // the concurrency limit may have grown
    }

    pub async fn probe(&self, url: &str) -> Result<VideoInfo, EngineError> {
        if !ytdlp::is_youtube_url(url) {
            return Err(EngineError::InvalidUrl);
        }
        let tools = &self.0.tools;
        if !tools.ytdlp.exists() {
            return Err(EngineError::YtdlpMissing);
        }
        let out = runner::output(&tools.ytdlp, &ytdlp::probe_args(url.trim(), tools))
            .await
            .map_err(|e| EngineError::Probe(e.summary()))?;
        ytdlp::parse_probe(&out).map_err(EngineError::Probe)
    }

    pub fn add(&self, new: NewJob) -> Result<Job, EngineError> {
        match &new.source {
            JobSource::Youtube { url } if ytdlp::is_youtube_url(url) => {}
            _ => return Err(EngineError::InvalidUrl),
        }
        Ok(self.insert(new))
    }

    fn insert(&self, new: NewJob) -> Job {
        let job = self.0.state.lock().unwrap().add(new, now_ms());
        self.save();
        self.notify();
        self.pump();
        // pump may have changed the state within this call; return the latest.
        self.list()
            .into_iter()
            .find(|j| j.id == job.id)
            .unwrap_or(job)
    }

    /// CLI `transcode`: a job converting a local file. The source is only read.
    pub async fn add_file(
        &self,
        path: PathBuf,
        output_dir: Option<PathBuf>,
        preset: &str,
        quality: Option<String>,
    ) -> Result<Job, EngineError> {
        if !path.is_file() {
            return Err(EngineError::SourceNotFound(path.display().to_string()));
        }
        let preset = preset
            .parse::<Preset>()
            .ok()
            .filter(|p| *p != Preset::Original)
            .ok_or_else(|| EngineError::BadPreset(preset.to_string()))?;
        let quality = quality.unwrap_or_else(|| "original".into());
        let max_height = match quality.as_str() {
            "original" => None,
            q => Some(
                q.strip_suffix('p')
                    .and_then(|n| n.parse::<u32>().ok())
                    .filter(|h| *h > 0)
                    .ok_or_else(|| EngineError::BadQuality(quality.clone()))?,
            ),
        };
        // Audio presets have no resolution: show "audio" as the quality.
        let (quality, max_height) = if preset.is_audio_only() {
            ("audio".to_string(), None)
        } else {
            (quality, max_height)
        };
        let duration_secs = self.media_duration(&path).await?;
        let title = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file")
            .to_string();
        Ok(self.insert(NewJob {
            source: JobSource::File { path, output_dir },
            title,
            thumbnail: None,
            duration_secs,
            quality_label: quality,
            options: JobOptions {
                format_id: None,
                preset,
                subtitles: vec![],
                auto_subtitles: false,
                max_height,
            },
        }))
    }

    /// Reads the duration with ffprobe. Unreadable means not media. N/A duration is None.
    async fn media_duration(&self, path: &Path) -> Result<Option<f64>, EngineError> {
        let args: Vec<String> = [
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "csv=p=0",
            "-i",
        ]
        .iter()
        .map(|s| s.to_string())
        .chain([path.display().to_string()])
        .collect();
        let out = runner::output(&self.0.tools.ffprobe, &args)
            .await
            .map_err(|e| EngineError::InvalidMedia(e.summary()))?;
        Ok(out.trim().parse::<f64>().ok())
    }

    /// CLI path: probe → resolve quality/preset/subtitles → add. Omitted options use the settings defaults.
    pub async fn add_url(
        &self,
        url: &str,
        quality: Option<String>,
        preset: Option<String>,
        subs: Option<Vec<String>>,
    ) -> Result<Job, EngineError> {
        let cfg = self.config();
        let preset = match preset {
            Some(p) => p.parse::<Preset>().map_err(|_| EngineError::BadPreset(p))?,
            None => cfg.default_preset,
        };
        let want = quality.unwrap_or(cfg.default_quality);
        if want != "best"
            && want != "audio"
            && want
                .strip_suffix('p')
                .and_then(|n| n.parse::<u32>().ok())
                .is_none()
        {
            return Err(EngineError::BadQuality(want));
        }
        let info = self.probe(url).await?;
        let quality = if preset.is_audio_only() {
            None
        } else {
            ytdlp::resolve_quality(&info.qualities, &want).map_err(EngineError::BadQuality)?
        };
        let langs = subs.unwrap_or(cfg.default_subtitles);
        let (subtitles, auto_subtitles) = ytdlp::pick_subtitles(&info, &langs);
        self.add(NewJob {
            source: JobSource::Youtube {
                url: url.trim().to_string(),
            },
            title: info.title,
            thumbnail: info.thumbnail,
            duration_secs: info.duration_secs,
            quality_label: quality
                .as_ref()
                .map_or("audio".to_string(), |q| q.label.clone()),
            options: JobOptions {
                format_id: quality.map(|q| q.format_id),
                preset,
                subtitles,
                auto_subtitles,
                max_height: None,
            },
        })
    }

    pub fn stop(&self, id: u64) -> Result<(), EngineError> {
        {
            let mut st = self.0.state.lock().unwrap();
            let job = st.get_mut(id).ok_or(EngineError::NotFound(id))?;
            if !job.state.is_pending() {
                return Err(EngineError::NotStoppable(id));
            }
            job.state = JobState::Stopped;
            job.speed = None;
            job.eta = None;
        }
        if let Some(t) = self.0.running.lock().unwrap().get(&id) {
            t.cancel();
        }
        self.save();
        self.notify();
        Ok(())
    }

    pub fn remove(&self, id: u64) -> Result<(), EngineError> {
        let removed = self.0.state.lock().unwrap().remove(id);
        let job = removed.ok_or(EngineError::NotFound(id))?;
        let token = self.0.running.lock().unwrap().get(&id).cloned();
        match (token, job.work_dir) {
            (Some(t), _) => t.cancel(), // the job task removes the package as it ends
            (None, Some(dir)) => {
                files::remove_part_dir(&dir);
            }
            (None, None) => {}
        }
        self.save();
        self.notify();
        Ok(())
    }

    /// Removes all completed jobs from the list, leaving their files alone. Returns how many.
    pub fn clear_completed(&self) -> usize {
        let removed = {
            let mut st = self.0.state.lock().unwrap();
            let before = st.jobs.len();
            st.jobs.retain(|j| j.state != JobState::Completed);
            before - st.jobs.len()
        };
        if removed > 0 {
            self.save();
            self.notify();
        }
        removed
    }

    pub fn restart(&self, id: u64) -> Result<(), EngineError> {
        {
            let mut st = self.0.state.lock().unwrap();
            let job = st.get_mut(id).ok_or(EngineError::NotFound(id))?;
            if !matches!(job.state, JobState::Stopped | JobState::Failed(_)) {
                return Err(EngineError::NotRestartable(id));
            }
            job.state = JobState::Queued;
            job.progress = 0.0;
            job.output = None;
            job.speed = None;
            job.eta = None;
        }
        self.save();
        self.notify();
        self.pump();
        Ok(())
    }

    /// Right before the app exits: requeues running jobs (resumed on next launch), cancels
    /// child processes and waits up to `timeout` for cleanup. No job starts afterwards.
    /// Blocking call.
    pub fn shutdown_for_exit(&self, timeout: Duration) {
        self.0.shutting_down.store(true, Ordering::SeqCst);
        {
            let mut st = self.0.state.lock().unwrap();
            for job in st.jobs.iter_mut().filter(|j| j.state.is_active()) {
                job.state = JobState::Queued;
                job.progress = 0.0;
                job.speed = None;
                job.eta = None;
            }
        }
        self.save(); // save first so a forced kill while waiting still leaves jobs queued
        for token in self.0.running.lock().unwrap().values() {
            token.cancel();
        }
        let deadline = Instant::now() + timeout;
        while !self.0.running.lock().unwrap().is_empty() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        self.save();
    }

    fn save(&self) {
        let st = self.0.state.lock().unwrap();
        if let Err(e) = self.0.paths.store.save(&st) {
            log::error!("queue save failed: {e}");
        }
    }

    /// Snapshot + listener call in one critical section. Never call it while holding
    /// the `state`/`running` locks (lock order: notify_lock → state).
    fn notify(&self) {
        let _serial = self.0.notify_lock.lock().unwrap();
        let jobs = self.list();
        (self.0.listener)(&jobs);
    }

    /// Starts as many queued jobs as there are free slots.
    fn pump(&self) {
        loop {
            if self.0.shutting_down.load(Ordering::SeqCst) {
                return;
            }
            let cfg = self.config();
            let max = cfg.max_concurrent.max(1);
            let next = {
                let mut running = self.0.running.lock().unwrap();
                if running.len() >= max {
                    return;
                }
                let mut st = self.0.state.lock().unwrap();
                let Some(id) = st.next_queued(|id| running.contains_key(&id)) else {
                    return;
                };
                let job = st.get_mut(id).expect("queued job exists");
                job.state = match job.source {
                    JobSource::File { .. } => JobState::Encoding,
                    JobSource::Youtube { .. } => JobState::Downloading,
                };
                job.progress = 0.0;
                let out_dir = job.result_dir(&cfg.download_dir);
                // No saved package (removed or never created) or no marker: create a new one in the output folder.
                let work_dir = match &job.work_dir {
                    Some(d) if files::is_part_dir(d) => d.clone(),
                    _ => {
                        let name = match &job.source {
                            JobSource::Youtube { .. } => {
                                format!("{}.{}", files::safe_title(&job.title), files::PART_EXT)
                            }
                            JobSource::File { path, .. } => {
                                let stem = files::safe_title(
                                    path.file_stem().and_then(|s| s.to_str()).unwrap_or("file"),
                                );
                                let n = files::next_variant_index(&out_dir, &stem);
                                format!("{stem}-{n}.{}", files::PART_EXT)
                            }
                        };
                        let dir = files::unique_path(&out_dir, &name);
                        // Create it now to claim the name (the next job with the same title gets another).
                        // On failure nothing is saved; the pipeline reports an output-folder error.
                        if files::make_part_dir(&dir).is_ok() {
                            job.work_dir = Some(dir.clone());
                        }
                        dir
                    }
                };
                let job = job.clone();
                let token = CancellationToken::new();
                running.insert(id, token.clone());
                (job, token, work_dir)
            };
            self.save();
            self.notify();
            self.spawn_job(next.0, next.1, next.2);
        }
    }

    fn spawn_job(&self, job: Job, token: CancellationToken, work_dir: PathBuf) {
        let me = self.clone();
        self.0.rt.spawn(async move {
            let cfg = me.config();
            let pcfg = PipelineCfg {
                tools: me.0.tools.clone(),
                hw_accel: cfg.hw_accel,
                download_dir: job.result_dir(&cfg.download_dir),
                work_dir,
                log_path: me.0.paths.log_dir.join(format!("{}.log", job.id)),
            };
            let id = job.id;
            let reporter = me.clone();
            let result =
                pipeline::run(&job, &pcfg, &token, move |r| reporter.on_report(id, r)).await;
            me.finish(id, Some(&pcfg.work_dir), result);
        });
    }

    fn on_report(&self, id: u64, r: Report) {
        let stage_changed = {
            let mut st = self.0.state.lock().unwrap();
            let Some(job) = st.get_mut(id) else { return };
            if !job.state.is_active() {
                return; // already stopped or removed
            }
            let next = match r.stage {
                Stage::Downloading => JobState::Downloading,
                Stage::Encoding => JobState::Encoding,
            };
            let changed = job.state != next;
            job.state = next;
            job.progress = r.progress;
            job.speed = r.speed;
            job.eta = r.eta;
            changed
        };
        if stage_changed {
            self.save();
        }
        self.notify();
    }

    fn finish(&self, id: u64, work_dir: Option<&Path>, result: Result<PathBuf, PipelineError>) {
        let still_listed = {
            let mut st = self.0.state.lock().unwrap();
            match st.get_mut(id) {
                None => false,
                Some(job) => {
                    match &result {
                        // The file really exists. Even if stopped or quitting meanwhile it stays completed,
                        // so a restart never downloads the same video again.
                        Ok(path) => {
                            job.state = JobState::Completed;
                            job.progress = 1.0;
                            job.output = Some(path.clone());
                            job.work_dir = None;
                        }
                        Err(_) if !job.state.is_active() => {}
                        Err(PipelineError::Cancelled) => job.state = JobState::Stopped,
                        Err(PipelineError::Failed(msg)) => {
                            job.state = JobState::Failed(msg.clone())
                        }
                    }
                    job.speed = None;
                    job.eta = None;
                    true
                }
            }
        };
        // Stopped or failed jobs keep their package so a restart resumes.
        if let Some(dir) = work_dir.filter(|_| !still_listed || result.is_ok()) {
            files::remove_part_dir(dir);
        }
        self.0.running.lock().unwrap().remove(&id);
        self.save();
        self.notify();
        self.pump();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queue::JsonQueueStore;
    use crate::testutil::{self, *};
    use std::{fs, path::Path, time::Duration};

    const SLOW_DOWNLOAD: &str = "sleep 1\n";

    struct Env {
        d: tempfile::TempDir,
        engine: Engine,
    }

    fn engine_at(dir: &Path, ytdlp_body: &str, max: usize) -> Engine {
        engine_with_listener(dir, ytdlp_body, max, |_| {})
    }

    fn engine_with_listener(
        dir: &Path,
        ytdlp_body: &str,
        max: usize,
        listener: impl Fn(&[Job]) + Send + Sync + 'static,
    ) -> Engine {
        let bin = dir.join("bin");
        fs::create_dir_all(&bin).unwrap();
        let tools = testutil::tools(&bin, &ytdlp_with_probe(ytdlp_body), FAKE_FFMPEG);
        let paths = EnginePaths {
            store: Arc::new(JsonQueueStore {
                path: dir.join("data/queue.json"),
            }),
            cache_dir: dir.join("cache"),
            log_dir: dir.join("logs"),
        };
        let cfg = EngineConfig {
            max_concurrent: max,
            hw_accel: true,
            download_dir: dir.join("Movies/kiri"),
            default_quality: "best".into(),
            default_preset: Preset::Original,
            default_subtitles: vec!["ko".into()],
        };
        Engine::new(paths, tools, cfg, Handle::current(), listener)
    }

    fn env(ytdlp_body: &str, max: usize) -> Env {
        let d = tempfile::tempdir().unwrap();
        let engine = engine_at(d.path(), ytdlp_body, max);
        engine.start();
        Env { d, engine }
    }

    fn new_job(title: &str) -> NewJob {
        NewJob {
            source: JobSource::Youtube {
                url: "https://youtu.be/abc123".into(),
            },
            title: title.into(),
            thumbnail: None,
            duration_secs: Some(2.0),
            quality_label: "720p".into(),
            options: JobOptions {
                format_id: Some("136".into()),
                preset: Preset::Original,
                subtitles: vec![],
                auto_subtitles: false,
                max_height: None,
            },
        }
    }

    async fn wait_for(e: &Engine, pred: impl Fn(&[Job]) -> bool) {
        for _ in 0..100 {
            if pred(&e.list()) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("timeout waiting; jobs = {:#?}", e.list());
    }

    fn state_of(e: &Engine, id: u64) -> JobState {
        e.list().into_iter().find(|j| j.id == id).unwrap().state
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn probe_rejects_non_youtube_without_running_ytdlp() {
        let e = env("touch \"$0.ran\"", 2);
        assert!(matches!(
            e.engine.probe("hello").await,
            Err(EngineError::InvalidUrl)
        ));
        assert!(matches!(
            e.engine.probe("https://vimeo.com/1").await,
            Err(EngineError::InvalidUrl)
        ));
        assert!(!e.engine.tools().ytdlp.with_extension("ran").exists());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn probe_requires_ytdlp() {
        let d = tempfile::tempdir().unwrap();
        let engine = engine_at(d.path(), "", 1);
        fs::remove_file(&engine.tools().ytdlp).unwrap();
        assert!(matches!(
            engine.probe("https://youtu.be/x").await,
            Err(EngineError::YtdlpMissing)
        ));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn probe_parses_fixture() {
        let e = env(FAKE_YTDLP_DOWNLOAD, 2);
        let info = e.engine.probe("https://youtu.be/abc123").await.unwrap();
        assert_eq!(info.title, "Rust in 100 Seconds");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn added_job_runs_to_completion() {
        let e = env(FAKE_YTDLP_DOWNLOAD, 2);
        let job = e.engine.add(new_job("a")).unwrap();
        wait_for(&e.engine, |j| j[0].state == JobState::Completed).await;
        let done = &e.engine.list()[0];
        assert_eq!(done.id, job.id);
        assert_eq!(
            done.output.as_deref(),
            Some(e.d.path().join("Movies/kiri/Fake Video.mp4").as_path())
        );
        assert!(
            !e.d.path().join("Movies/kiri/a.kiripart").exists(),
            "package cleaned after success"
        );
        assert_eq!(done.work_dir, None);
    }

    fn work_dir_of(e: &Engine, id: u64) -> PathBuf {
        e.list()
            .into_iter()
            .find(|j| j.id == id)
            .unwrap()
            .work_dir
            .expect("work_dir assigned")
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn running_job_downloads_into_kiripart_package() {
        let e = env("sleep 30", 2);
        e.engine.add(new_job("Rust in 100 Seconds")).unwrap();
        let wd = work_dir_of(&e.engine, 1);
        assert_eq!(
            wd,
            e.d.path().join("Movies/kiri/Rust in 100 Seconds.kiripart")
        );
        assert!(wd.join(".kiri").is_file());
        // It must also stay in the saved queue to resume on the next launch.
        let saved = QueueState::load(&e.d.path().join("data/queue.json"));
        assert_eq!(saved.jobs[0].work_dir.as_deref(), Some(wd.as_path()));
        e.engine.stop(1).unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn restart_after_package_deleted_gets_fresh_path_in_current_dir() {
        let e = env("sleep 30", 2);
        e.engine.add(new_job("a")).unwrap();
        let wd = work_dir_of(&e.engine, 1);
        e.engine.stop(1).unwrap();
        wait_for(&e.engine, |_| e.engine.0.running.lock().unwrap().is_empty()).await;
        fs::remove_dir_all(&wd).unwrap();
        let mut cfg = e.engine.config();
        cfg.download_dir = e.d.path().join("Elsewhere");
        e.engine.set_config(cfg);
        e.engine.restart(1).unwrap();
        let fresh = work_dir_of(&e.engine, 1);
        assert_eq!(fresh, e.d.path().join("Elsewhere/a.kiripart"));
        assert!(fresh.join(".kiri").is_file());
        e.engine.stop(1).unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn job_with_missing_old_download_dir_gets_new_path() {
        let d = tempfile::tempdir().unwrap();
        let mut q = QueueState::default();
        q.add(new_job("a"), 0);
        q.get_mut(1).unwrap().state = JobState::Stopped;
        q.get_mut(1).unwrap().work_dir = Some(d.path().join("Gone/a.kiripart"));
        q.save(&d.path().join("data/queue.json")).unwrap();
        let engine = engine_at(d.path(), "sleep 30", 1);
        engine.start();
        engine.restart(1).unwrap();
        assert_eq!(
            work_dir_of(&engine, 1),
            d.path().join("Movies/kiri/a.kiripart")
        );
        engine.stop(1).unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn remove_stopped_job_deletes_package() {
        let e = env("sleep 30", 2);
        e.engine.add(new_job("a")).unwrap();
        let wd = work_dir_of(&e.engine, 1);
        e.engine.stop(1).unwrap();
        // Takes the path that removes it once the task has fully ended (idle).
        wait_for(&e.engine, |_| e.engine.0.running.lock().unwrap().is_empty()).await;
        assert!(wd.exists());
        e.engine.remove(1).unwrap();
        assert!(!wd.exists());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn failed_job_keeps_package() {
        let e = env("echo 'ERROR: boom' >&2; exit 1", 2);
        e.engine.add(new_job("a")).unwrap();
        let wd = work_dir_of(&e.engine, 1);
        wait_for(&e.engine, |j| matches!(j[0].state, JobState::Failed(_))).await;
        assert!(wd.join(".kiri").is_file());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn same_title_jobs_get_distinct_packages() {
        let e = env("sleep 30", 2);
        e.engine.add(new_job("a")).unwrap();
        e.engine.add(new_job("a")).unwrap();
        let kiri = e.d.path().join("Movies/kiri");
        assert_eq!(work_dir_of(&e.engine, 1), kiri.join("a.kiripart"));
        assert_eq!(work_dir_of(&e.engine, 2), kiri.join("a (1).kiripart"));
        e.engine.stop(1).unwrap();
        e.engine.stop(2).unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn restart_reuses_work_dir_even_if_download_dir_changed() {
        let e = env("sleep 30", 2);
        e.engine.add(new_job("a")).unwrap();
        let wd = work_dir_of(&e.engine, 1);
        e.engine.stop(1).unwrap();
        wait_for(&e.engine, |_| !e.engine.has_active()).await;
        assert!(wd.join(".kiri").exists(), "stopped job keeps its package");
        let mut cfg = e.engine.config();
        cfg.download_dir = e.d.path().join("Elsewhere");
        e.engine.set_config(cfg);
        e.engine.restart(1).unwrap();
        assert_eq!(work_dir_of(&e.engine, 1), wd);
        e.engine.stop(1).unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn respects_max_concurrent() {
        let e = env(&format!("{SLOW_DOWNLOAD}{FAKE_YTDLP_DOWNLOAD}"), 1);
        e.engine.add(new_job("a")).unwrap();
        e.engine.add(new_job("b")).unwrap();
        assert_eq!(state_of(&e.engine, 1), JobState::Downloading);
        assert_eq!(state_of(&e.engine, 2), JobState::Queued);
        wait_for(&e.engine, |j| {
            j.iter().all(|j| j.state == JobState::Completed)
        })
        .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn last_listener_snapshot_is_the_newest_state() {
        let d = tempfile::tempdir().unwrap();
        let seen: Arc<Mutex<Vec<Vec<Job>>>> = Arc::new(Mutex::new(Vec::new()));
        let rec = seen.clone();
        let overlap = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let inside = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let (saw_overlap, counter) = (overlap.clone(), inside.clone());
        let engine = engine_with_listener(d.path(), FAKE_YTDLP_DOWNLOAD, 2, move |jobs| {
            use std::sync::atomic::Ordering::SeqCst;
            // A slow listener (an IPC emit in practice). Without serialization two notifications
            // overlap and the older snapshot arrives last.
            if counter.fetch_add(1, SeqCst) > 0 {
                saw_overlap.store(true, SeqCst);
            }
            std::thread::sleep(Duration::from_millis(20));
            rec.lock().unwrap().push(jobs.to_vec());
            counter.fetch_sub(1, SeqCst);
        });
        engine.start();
        engine.add(new_job("a")).unwrap();
        engine.add(new_job("b")).unwrap();
        wait_for(&engine, |j| {
            j.len() == 2 && j.iter().all(|j| j.state == JobState::Completed)
        })
        .await;
        tokio::time::sleep(Duration::from_millis(500)).await; // time for pending notifications to go out
        let snapshots = seen.lock().unwrap().clone();
        assert!(!snapshots.is_empty(), "listener never called");
        assert!(
            !overlap.load(std::sync::atomic::Ordering::SeqCst),
            "listener called concurrently; snapshots can arrive out of order"
        );
        let last = snapshots.last().unwrap();
        assert_eq!(last.len(), 2);
        assert!(
            last.iter().all(|j| j.state == JobState::Completed),
            "stale snapshot emitted last: {last:#?}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn stop_running_job() {
        let e = env("sleep 30", 2);
        e.engine.add(new_job("a")).unwrap();
        e.engine.stop(1).unwrap();
        assert_eq!(state_of(&e.engine, 1), JobState::Stopped);
        wait_for(&e.engine, |_| !e.engine.has_active()).await;
        assert_eq!(state_of(&e.engine, 1), JobState::Stopped);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn stop_queued_job_and_reject_finished() {
        let e = env("sleep 30", 1);
        e.engine.add(new_job("a")).unwrap();
        e.engine.add(new_job("b")).unwrap();
        e.engine.stop(2).unwrap();
        assert_eq!(state_of(&e.engine, 2), JobState::Stopped);
        assert!(matches!(
            e.engine.stop(2),
            Err(EngineError::NotStoppable(2))
        ));
        assert!(matches!(e.engine.stop(99), Err(EngineError::NotFound(99))));
        e.engine.stop(1).unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn stop_then_immediate_restart_completes_once() {
        let e = env(&format!("{SLOW_DOWNLOAD}{FAKE_YTDLP_DOWNLOAD}"), 2);
        e.engine.add(new_job("a")).unwrap();
        e.engine.stop(1).unwrap();
        e.engine.restart(1).unwrap(); // the previous process may still be exiting
        wait_for(&e.engine, |j| j[0].state == JobState::Completed).await;
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert_eq!(state_of(&e.engine, 1), JobState::Completed);
        let outputs = fs::read_dir(e.d.path().join("Movies/kiri"))
            .unwrap()
            .filter(|f| {
                f.as_ref()
                    .unwrap()
                    .path()
                    .extension()
                    .is_some_and(|x| x == "mp4")
            })
            .count();
        assert_eq!(outputs, 1, "job ran twice");
        assert!(matches!(
            e.engine.restart(1),
            Err(EngineError::NotRestartable(1))
        ));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn remove_running_job_cleans_up() {
        let e = env("sleep 30", 2);
        e.engine.add(new_job("a")).unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;
        let wd = e.d.path().join("Movies/kiri/a.kiripart");
        assert!(wd.exists());
        e.engine.remove(1).unwrap();
        assert!(e.engine.list().is_empty());
        wait_for(&e.engine, |_| !wd.exists()).await;
        assert!(e.engine.list().is_empty(), "removed job resurrected");
        assert!(matches!(e.engine.remove(1), Err(EngineError::NotFound(1))));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn failed_job_keeps_message_and_can_restart() {
        let e = env("echo 'ERROR: boom' >&2; exit 1", 2);
        e.engine.add(new_job("a")).unwrap();
        wait_for(&e.engine, |j| matches!(j[0].state, JobState::Failed(_))).await;
        assert_eq!(
            state_of(&e.engine, 1),
            JobState::Failed("ERROR: boom".into())
        );
        e.engine.restart(1).unwrap();
        wait_for(&e.engine, |j| matches!(j[0].state, JobState::Failed(_))).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn add_url_resolves_quality_preset_and_subs() {
        let e = env("sleep 30", 1);
        let j = e
            .engine
            .add_url(
                "https://youtu.be/abc123",
                Some("720p".into()),
                None,
                Some(vec!["ja".into()]),
            )
            .await
            .unwrap();
        assert_eq!(j.options.format_id.as_deref(), Some("136"));
        assert_eq!(j.quality_label, "720p");
        assert_eq!(
            (j.options.subtitles.clone(), j.options.auto_subtitles),
            (vec!["ja".to_string()], true)
        );
        assert_eq!(j.title, "Rust in 100 Seconds");
        let a = e
            .engine
            .add_url("https://youtu.be/abc123", None, Some("mp3".into()), None)
            .await
            .unwrap();
        assert_eq!(
            (
                a.options.format_id,
                a.quality_label.as_str(),
                a.options.preset
            ),
            (None, "audio", Preset::Mp3)
        );
        assert_eq!(a.options.subtitles, vec!["ko"]); // default subtitles
        assert!(matches!(
            e.engine
                .add_url("https://youtu.be/x", None, Some("avi".into()), None)
                .await,
            Err(EngineError::BadPreset(_))
        ));
        assert!(matches!(
            e.engine
                .add_url("https://youtu.be/x", Some("hd".into()), None, None)
                .await,
            Err(EngineError::BadQuality(_))
        ));
        for j in e.engine.list() {
            let _ = e.engine.stop(j.id);
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn shutdown_for_exit_requeues_and_waits_for_children() {
        let e = env("sleep 30", 2);
        e.engine.add(new_job("a")).unwrap();
        wait_for(&e.engine, |_| e.engine.has_active()).await;
        let engine = e.engine.clone();
        tokio::task::spawn_blocking(move || engine.shutdown_for_exit(Duration::from_secs(2)))
            .await
            .unwrap();
        assert!(e.engine.0.running.lock().unwrap().is_empty());
        assert_eq!(state_of(&e.engine, 1), JobState::Queued);
        // A cancelled job ending must not make pump start it again.
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(e.engine.0.running.lock().unwrap().is_empty());
        assert_eq!(state_of(&e.engine, 1), JobState::Queued);
        let saved = QueueState::load(&e.d.path().join("data/queue.json"));
        assert_eq!(saved.jobs[0].state, JobState::Queued);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn new_engine_recovers_persisted_jobs() {
        let d = tempfile::tempdir().unwrap();
        let mut q = QueueState::default();
        q.add(new_job("a"), 0);
        q.get_mut(1).unwrap().state = JobState::Downloading;
        q.save(&d.path().join("data/queue.json")).unwrap();
        let engine = engine_at(d.path(), "sleep 30", 1); // start() is not called
        assert_eq!(state_of(&engine, 1), JobState::Queued);
        assert_eq!(engine.add(new_job("b")).unwrap().id, 2);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn finish_ok_completes_even_after_stop() {
        let d = tempfile::tempdir().unwrap();
        let mut q = QueueState::default();
        q.add(new_job("a"), 0);
        q.get_mut(1).unwrap().state = JobState::Stopped;
        q.save(&d.path().join("data/queue.json")).unwrap();
        let engine = engine_at(d.path(), "sleep 30", 1);
        let out = d.path().join("Movies/kiri/a.mp4");
        engine.finish(1, None, Ok(out.clone()));
        let j = &engine.list()[0];
        assert_eq!(j.state, JobState::Completed);
        assert_eq!(
            (j.progress, j.output.as_deref()),
            (1.0, Some(out.as_path()))
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn restart_clears_previous_output() {
        let d = tempfile::tempdir().unwrap();
        let mut q = QueueState::default();
        q.add(new_job("a"), 0);
        let j = q.get_mut(1).unwrap();
        j.state = JobState::Failed("x".into());
        j.output = Some("/old.mp4".into());
        j.speed = Some("1.0 MB/s".into());
        j.eta = Some("00:01".into());
        q.save(&d.path().join("data/queue.json")).unwrap();
        let engine = engine_at(d.path(), "sleep 30", 1);
        engine.0.shutting_down.store(true, Ordering::SeqCst); // so pump starts nothing
        engine.restart(1).unwrap();
        let j = &engine.list()[0];
        assert_eq!(j.state, JobState::Queued);
        assert_eq!((&j.output, &j.speed, &j.eta), (&None, &None, &None));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn clear_completed_removes_only_completed_and_keeps_files() {
        let d = tempfile::tempdir().unwrap();
        let out = d.path().join("Movies/kiri/a.mp4");
        std::fs::create_dir_all(out.parent().unwrap()).unwrap();
        std::fs::write(&out, "x").unwrap();
        let mut q = QueueState::default();
        for t in ["a", "b", "c", "d", "e"] {
            q.add(new_job(t), 0);
        }
        q.get_mut(1).unwrap().state = JobState::Completed;
        q.get_mut(1).unwrap().output = Some(out.clone());
        q.get_mut(2).unwrap().state = JobState::Completed;
        q.get_mut(3).unwrap().state = JobState::Failed("x".into());
        q.get_mut(4).unwrap().state = JobState::Stopped;
        // 5 stays Queued
        q.save(&d.path().join("data/queue.json")).unwrap();
        let engine = engine_at(d.path(), "sleep 30", 1);
        engine.0.shutting_down.store(true, Ordering::SeqCst); // so pump starts nothing

        assert_eq!(engine.clear_completed(), 2);
        let ids: Vec<u64> = engine.list().iter().map(|j| j.id).collect();
        assert_eq!(ids, vec![3, 4, 5]);
        assert!(out.exists(), "downloaded file must stay");
        let saved = QueueState::load(&d.path().join("data/queue.json"));
        assert_eq!(saved.jobs.len(), 3, "persisted");
        assert_eq!(engine.clear_completed(), 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn add_rejects_non_youtube_url() {
        let e = env("sleep 30", 1);
        let mut j = new_job("a");
        j.source = JobSource::Youtube {
            url: "--exec=touch /tmp/pwned".into(),
        };
        assert!(matches!(e.engine.add(j), Err(EngineError::InvalidUrl)));
        assert!(e.engine.list().is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn new_engine_removes_legacy_cache_and_orphan_packages() {
        let d = tempfile::tempdir().unwrap();
        let kiri = d.path().join("Movies/kiri");
        let marked = |n: &str| {
            let p = kiri.join(n);
            fs::create_dir_all(&p).unwrap();
            fs::write(p.join(".kiri"), "").unwrap();
            p
        };
        let stopped = marked("s.kiripart");
        let queued = marked("q.kiripart");
        let orphan = marked("o.kiripart");
        let unmarked = kiri.join("foo.kiripart");
        fs::create_dir_all(&unmarked).unwrap();
        let mut q = QueueState::default();
        q.add(new_job("s"), 0);
        q.add(new_job("q"), 0);
        q.get_mut(1).unwrap().state = JobState::Stopped;
        q.get_mut(1).unwrap().work_dir = Some(stopped.clone());
        q.get_mut(2).unwrap().work_dir = Some(queued.clone());
        q.save(&d.path().join("data/queue.json")).unwrap();
        let legacy = d.path().join("cache/jobs");
        fs::create_dir_all(legacy.join("1")).unwrap();
        let _engine = engine_at(d.path(), "sleep 30", 1);
        assert!(!legacy.exists(), "legacy cache removed");
        assert!(
            stopped.exists() && queued.exists(),
            "referenced packages kept"
        );
        assert!(!orphan.exists(), "orphan package removed");
        assert!(unmarked.exists(), "package without marker is never deleted");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn orphan_cleanup_matches_referenced_paths_case_insensitively() {
        let d = tempfile::tempdir().unwrap();
        let kiri = d.path().join("Movies/kiri");
        let pkg = kiri.join("s.kiripart");
        files::make_part_dir(&pkg).unwrap();
        let other_case = d.path().join("MOVIES/KIRI/s.kiripart");
        if !other_case.exists() {
            return; // cannot be checked on a case-sensitive file system
        }
        let mut q = QueueState::default();
        q.add(new_job("s"), 0);
        q.get_mut(1).unwrap().state = JobState::Stopped;
        q.get_mut(1).unwrap().work_dir = Some(other_case);
        q.save(&d.path().join("data/queue.json")).unwrap();
        let _engine = engine_at(d.path(), "sleep 30", 1);
        assert!(pkg.exists(), "referenced package kept despite path casing");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn corrupt_queue_skips_orphan_cleanup() {
        let d = tempfile::tempdir().unwrap();
        let pkg = d.path().join("Movies/kiri/s.kiripart");
        files::make_part_dir(&pkg).unwrap();
        fs::create_dir_all(d.path().join("data")).unwrap();
        fs::write(d.path().join("data/queue.json"), "{nope").unwrap();
        let _engine = engine_at(d.path(), "sleep 30", 1);
        assert!(
            pkg.exists(),
            "corrupt queue must not wipe partial downloads"
        );
    }

    #[test]
    fn error_codes() {
        assert_eq!(EngineError::InvalidUrl.code(), "invalid_url");
        assert_eq!(EngineError::NotFound(3).code(), "not_found");
        assert_eq!(EngineError::NotFound(3).to_string(), "job 3 not found");
        assert_eq!(EngineError::YtdlpMissing.code(), "ytdlp_missing");
    }

    fn source_file(d: &Path, name: &str) -> PathBuf {
        let dir = d.join("clips");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join(name);
        std::fs::write(&p, "orig").unwrap();
        p
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn add_file_validates_input() {
        let e = env("sleep 30", 1);
        let missing = e.d.path().join("nope.mkv");
        assert!(matches!(
            e.engine.add_file(missing, None, "mp4-h264", None).await,
            Err(EngineError::SourceNotFound(_))
        ));
        assert!(matches!(
            e.engine
                .add_file(e.d.path().to_path_buf(), None, "mp4-h264", None)
                .await,
            Err(EngineError::SourceNotFound(_))
        ));
        let src = source_file(e.d.path(), "a.mkv");
        assert!(matches!(
            e.engine.add_file(src.clone(), None, "original", None).await,
            Err(EngineError::BadPreset(_))
        ));
        assert!(matches!(
            e.engine.add_file(src.clone(), None, "avi", None).await,
            Err(EngineError::BadPreset(_))
        ));
        assert!(matches!(
            e.engine
                .add_file(src.clone(), None, "mp4-h264", Some("hd".into()))
                .await,
            Err(EngineError::BadQuality(_))
        ));
        testutil::script(
            &e.d.path().join("bin"),
            "ffprobe",
            "echo 'Invalid data found when processing input' >&2; exit 1",
        );
        match e.engine.add_file(src, None, "mp4-h264", None).await {
            Err(EngineError::InvalidMedia(m)) => assert!(m.contains("Invalid data"), "{m}"),
            other => panic!("{other:?}"),
        }
        assert!(e.engine.list().is_empty());
        assert_eq!(
            EngineError::SourceNotFound("x".into()).code(),
            "source_not_found"
        );
        assert_eq!(
            EngineError::InvalidMedia("x".into()).code(),
            "invalid_media"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn add_file_audio_preset_labels_audio() {
        let e = env("sleep 30", 1);
        let src = source_file(e.d.path(), "a.mkv");
        for q in [None, Some("720p".to_string())] {
            let j = e
                .engine
                .add_file(src.clone(), None, "mp3", q)
                .await
                .unwrap();
            assert_eq!(
                (j.quality_label.as_str(), j.options.max_height),
                ("audio", None)
            );
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn add_rejects_file_source_from_ui_path() {
        let e = env("sleep 30", 1);
        let mut j = new_job("a");
        j.source = JobSource::File {
            path: "/tmp/x.mkv".into(),
            output_dir: None,
        };
        assert!(matches!(e.engine.add(j), Err(EngineError::InvalidUrl)));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn file_transcode_keeps_source_and_writes_variant() {
        let e = env("sleep 30", 2);
        let src = source_file(e.d.path(), "clip.mkv");
        let before = std::fs::metadata(&src).unwrap().modified().unwrap();
        let j = e
            .engine
            .add_file(src.clone(), None, "mp4-h264", Some("720p".into()))
            .await
            .unwrap();
        assert_eq!(
            (
                j.title.as_str(),
                j.quality_label.as_str(),
                j.options.max_height,
                j.duration_secs
            ),
            ("clip.mkv", "720p", Some(720), Some(2.0))
        );
        wait_for(&e.engine, |jobs| jobs[0].state == JobState::Completed).await;
        let out = e.d.path().join("clips/clip-1.mp4");
        assert_eq!(e.engine.list()[0].output.as_deref(), Some(out.as_path()));
        assert!(out.exists());
        assert_eq!(std::fs::read_to_string(&src).unwrap(), "orig");
        assert_eq!(std::fs::metadata(&src).unwrap().modified().unwrap(), before);
        assert!(!e.d.path().join("clips/clip-1.kiripart").exists());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn file_transcode_honours_output_dir() {
        let e = env("sleep 30", 1);
        let src = source_file(e.d.path(), "clip.mkv");
        let out_dir = e.d.path().join("converted"); // a missing folder is created
        e.engine
            .add_file(src, Some(out_dir.clone()), "mp3", None)
            .await
            .unwrap();
        wait_for(&e.engine, |jobs| jobs[0].state == JobState::Completed).await;
        assert!(out_dir.join("clip-1.mp3").exists());
        assert!(!e.d.path().join("clips/clip-1.mp3").exists());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn two_transcodes_of_same_file_get_distinct_indices() {
        let e = env("sleep 30", 2);
        let src = source_file(e.d.path(), "clip.mkv");
        e.engine
            .add_file(src.clone(), None, "mp4-h264", None)
            .await
            .unwrap();
        e.engine
            .add_file(src, None, "mov-prores", None)
            .await
            .unwrap();
        wait_for(&e.engine, |jobs| {
            jobs.iter().all(|j| j.state == JobState::Completed)
        })
        .await;
        let clips = e.d.path().join("clips");
        let outs: Vec<_> = e
            .engine
            .list()
            .into_iter()
            .map(|j| j.output.unwrap())
            .collect();
        assert_ne!(outs[0], outs[1]);
        assert!(
            outs.iter()
                .all(|o| o.exists() && o.parent() == Some(clips.as_path()))
        );
        let mut names: Vec<String> = outs
            .iter()
            .map(|o| o.file_stem().unwrap().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(names, vec!["clip-1", "clip-2"]);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn source_deleted_before_start_fails_job() {
        let e = env("sleep 30", 1);
        let bin = e.d.path().join("bin");
        // fake ffmpeg: fails if the file after -i is missing
        testutil::script(
            &bin,
            "ffmpeg",
            &format!(
                "prev=\"\"\nfor a; do if [ \"$prev\" = \"-i\" ] && [ ! -f \"$a\" ]; then echo \"$a: No such file or directory\" >&2; exit 1; fi; prev=\"$a\"; done\n{FAKE_FFMPEG}"
            ),
        );
        e.engine.add(new_job("busy")).unwrap(); // occupies the slot (sleep 30)
        let src = source_file(e.d.path(), "clip.mkv");
        e.engine
            .add_file(src.clone(), None, "mp4-h264", None)
            .await
            .unwrap();
        std::fs::remove_file(&src).unwrap();
        e.engine.stop(1).unwrap(); // frees the slot so the transcode job starts
        wait_for(&e.engine, |jobs| {
            matches!(jobs[1].state, JobState::Failed(_))
        })
        .await;
        let JobState::Failed(m) = &e.engine.list()[1].state else {
            unreachable!()
        };
        assert!(m.contains("No such file"), "{m}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn restart_file_job_reencodes_from_scratch() {
        let e = env("sleep 30", 1);
        let bin = e.d.path().join("bin");
        let marker = e.d.path().join("second-run");
        // First run: write "partial" to the output and wait. Second (marker present): complete normally.
        testutil::script(
            &bin,
            "ffmpeg",
            &format!(
                "for a; do last=\"$a\"; done\nif [ ! -f '{}' ]; then mkdir -p \"$(dirname \"$last\")\"; echo partial > \"$last\"; sleep 30; fi\n{FAKE_FFMPEG}",
                marker.display()
            ),
        );
        let src = source_file(e.d.path(), "clip.mkv");
        e.engine
            .add_file(src, None, "mp4-h264", None)
            .await
            .unwrap();
        wait_for(&e.engine, |jobs| jobs[0].state == JobState::Encoding).await;
        let pkg = e.engine.list()[0].work_dir.clone().unwrap();
        e.engine.stop(1).unwrap();
        // Wait until the previous task has really ended (the running map is empty).
        wait_for(&e.engine, |_| {
            e.engine.0.running.lock().unwrap().is_empty() && !e.engine.has_active()
        })
        .await;
        std::fs::write(&marker, "").unwrap();
        e.engine.restart(1).unwrap();
        wait_for(&e.engine, |jobs| jobs[0].state == JobState::Completed).await;
        let out = e.engine.list()[0].output.clone().unwrap();
        assert_eq!(out, e.d.path().join("clips/clip-1.mp4"));
        assert_eq!(
            std::fs::read_to_string(&out).unwrap(),
            "",
            "must not keep the partial output"
        );
        assert!(!pkg.exists());
    }
}
