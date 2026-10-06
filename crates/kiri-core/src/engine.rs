//! 앱 하나에 하나. 큐 상태 + 동시 실행 + 프로세스 취소를 묶는다. UI 와 CLI 가 같은 Engine 을 쓴다.
use crate::{
    files,
    model::{Job, JobOptions, JobState, NewJob, Preset, Tools},
    pipeline::{self, PipelineCfg, PipelineError, Report, Stage},
    queue::QueueState,
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
    /// CLI add 의 기본값 ("best" | "audio" | "<N>p")
    pub default_quality: String,
    pub default_preset: Preset,
    pub default_subtitles: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct EnginePaths {
    pub queue_file: PathBuf,
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
}

impl EngineError {
    /// 프론트엔드 i18n 키(error.<code>)와 CLI 응답에 쓰는 고정 코드.
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
    /// 스냅샷 생성과 리스너 호출을 묶어 직렬화한다. 이게 없으면 늦게 만든
    /// 스냅샷이 먼저 나가서 UI 가 낡은 상태에 멈춘다.
    notify_lock: Mutex<()>,
    /// 앱 종료 중. 더 이상 작업을 시작하지 않는다.
    shutting_down: AtomicBool,
}

#[derive(Clone)]
pub struct Engine(Arc<Inner>);

impl Engine {
    /// `listener` 는 큐가 바뀔 때마다 최신 목록으로 호출된다. 호출은 항상 직렬이고
    /// 순서가 보장된다. 그 대신 리스너 안에서 Engine 의 변경 메서드(add/stop/…)를
    /// 동기적으로 부르면 안 된다(재진입 교착).
    pub fn new(
        paths: EnginePaths,
        tools: Tools,
        config: EngineConfig,
        rt: Handle,
        listener: impl Fn(&[Job]) + Send + Sync + 'static,
    ) -> Engine {
        let mut state = QueueState::load(&paths.queue_file);
        state.recover();
        // 예전 버전의 작업 캐시. 받던 작업은 처음부터 다시 받는다.
        let _ = std::fs::remove_dir_all(paths.cache_dir.join("jobs"));
        // 큐에 없는 패키지(삭제 도중 종료 등)는 다시 쓰일 일이 없다. 표식이 있는 것만 지운다.
        if let Ok(entries) = std::fs::read_dir(&config.download_dir) {
            for e in entries.flatten() {
                let p = e.path();
                let ours = p.extension().is_some_and(|x| x == files::PART_EXT)
                    && p.join(files::PART_MARKER).is_file();
                let listed = state
                    .jobs
                    .iter()
                    .any(|j| j.work_dir.as_deref() == Some(p.as_path()));
                if ours && !listed {
                    let _ = std::fs::remove_dir_all(&p);
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

    /// 대기 작업 실행을 시작한다. setup 이 끝난 뒤(리스너가 준비된 뒤) 한 번 부른다.
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

    /// 대기·실행 중인 작업이 하나도 없다.
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
        self.pump(); // 동시 실행 수가 늘었을 수 있다
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
        if !ytdlp::is_youtube_url(&new.url) {
            return Err(EngineError::InvalidUrl);
        }
        let job = self.0.state.lock().unwrap().add(new, now_ms());
        self.save();
        self.notify();
        self.pump();
        // pump 가 같은 호출 안에서 Downloading 으로 바꿨을 수 있다. 최신 상태를 돌려준다.
        Ok(self
            .list()
            .into_iter()
            .find(|j| j.id == job.id)
            .unwrap_or(job))
    }

    /// CLI 경로: probe → 화질/프리셋/자막 해석 → add. 생략한 옵션은 설정 기본값.
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
            url: url.trim().to_string(),
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
            (Some(t), _) => t.cancel(), // 작업 태스크가 끝나면서 패키지를 지운다
            (None, Some(dir)) => {
                let _ = std::fs::remove_dir_all(dir);
            }
            (None, None) => {}
        }
        self.save();
        self.notify();
        Ok(())
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

    /// 앱 종료 직전. 실행 중인 작업을 대기로 되돌리고(다음 실행 때 이어 받음) 자식
    /// 프로세스를 취소한 뒤 `timeout` 까지 정리를 기다린다. 이후로는 작업을 시작하지 않는다.
    /// 블로킹 호출이다.
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
        self.save(); // 기다리는 도중 강제 종료돼도 대기 상태로 남게 먼저 저장한다
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
        if let Err(e) = st.save(&self.0.paths.queue_file) {
            eprintln!("kiri queue: save failed: {e}");
        }
    }

    /// 스냅샷 + 리스너 호출을 한 임계 구역에서 처리한다. `state`/`running` 락을
    /// 잡은 채로 부르지 않는다(락 순서: notify_lock → state).
    fn notify(&self) {
        let _serial = self.0.notify_lock.lock().unwrap();
        let jobs = self.list();
        (self.0.listener)(&jobs);
    }

    /// 빈 슬롯만큼 대기 작업을 시작한다.
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
                job.state = JobState::Downloading;
                job.progress = 0.0;
                if job.work_dir.is_none() {
                    let dir = files::unique_path(
                        &cfg.download_dir,
                        &format!("{}.{}", files::safe_title(&job.title), files::PART_EXT),
                    );
                    // 바로 만들어 이름을 선점한다(같은 제목의 다음 작업이 다른 이름을 받게).
                    // 실패는 pipeline 이 저장 폴더 오류로 보고한다.
                    let _ = files::make_part_dir(&dir);
                    job.work_dir = Some(dir);
                }
                let job = job.clone();
                let token = CancellationToken::new();
                running.insert(id, token.clone());
                (job, token)
            };
            self.save();
            self.notify();
            self.spawn_job(next.0, next.1);
        }
    }

    fn spawn_job(&self, job: Job, token: CancellationToken) {
        let me = self.clone();
        self.0.rt.spawn(async move {
            let cfg = me.config();
            let pcfg = PipelineCfg {
                tools: me.0.tools.clone(),
                hw_accel: cfg.hw_accel,
                download_dir: cfg.download_dir,
                work_dir: job.work_dir.clone().expect("pump assigns work_dir"),
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
                return; // 이미 중지·삭제됨
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
                        // 파일이 실제로 생겼다. 그 사이 중지·종료됐어도 완료로 남겨야
                        // 재시작이 같은 영상을 또 받지 않는다.
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
        // 중지·실패는 패키지를 남겨 재시작 때 이어 받는다.
        if let Some(dir) = work_dir.filter(|_| !still_listed || result.is_ok()) {
            let _ = std::fs::remove_dir_all(dir);
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
            queue_file: dir.join("data/queue.json"),
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
            url: "https://youtu.be/abc123".into(),
            title: title.into(),
            thumbnail: None,
            duration_secs: Some(2.0),
            quality_label: "720p".into(),
            options: JobOptions {
                format_id: Some("136".into()),
                preset: Preset::Original,
                subtitles: vec![],
                auto_subtitles: false,
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
        // 저장된 큐에도 남아야 재실행 때 이어 받는다.
        let saved = QueueState::load(&e.d.path().join("data/queue.json"));
        assert_eq!(saved.jobs[0].work_dir.as_deref(), Some(wd.as_path()));
        e.engine.stop(1).unwrap();
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
            // 느린 리스너(실제로는 IPC emit). 직렬화돼 있지 않으면 두 통지가 겹치고,
            // 먼저 뜬 낡은 스냅샷이 가장 늦게 도착한다.
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
        tokio::time::sleep(Duration::from_millis(500)).await; // 남은 알림이 모두 나갈 시간
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
        e.engine.restart(1).unwrap(); // 이전 프로세스가 아직 종료 중일 수 있다
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
        assert_eq!(a.options.subtitles, vec!["ko"]); // 기본 자막
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
        // 취소된 작업이 끝나면서 pump 가 다시 시작하지 않아야 한다.
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
        let engine = engine_at(d.path(), "sleep 30", 1); // start() 를 부르지 않는다
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
        engine.0.shutting_down.store(true, Ordering::SeqCst); // pump 가 시작하지 않게
        engine.restart(1).unwrap();
        let j = &engine.list()[0];
        assert_eq!(j.state, JobState::Queued);
        assert_eq!((&j.output, &j.speed, &j.eta), (&None, &None, &None));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn add_rejects_non_youtube_url() {
        let e = env("sleep 30", 1);
        let mut j = new_job("a");
        j.url = "--exec=touch /tmp/pwned".into();
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

    #[test]
    fn error_codes() {
        assert_eq!(EngineError::InvalidUrl.code(), "invalid_url");
        assert_eq!(EngineError::NotFound(3).code(), "not_found");
        assert_eq!(EngineError::NotFound(3).to_string(), "job 3 not found");
        assert_eq!(EngineError::YtdlpMissing.code(), "ytdlp_missing");
    }
}
