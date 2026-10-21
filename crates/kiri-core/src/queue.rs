//! Plain queue state. The engine owns concurrency and processes; this handles the list and the file only.
use crate::model::{Job, JobState, NewJob};
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

/// Where the engine persists the queue. `save` is called while the engine holds its
/// state lock, so an implementation must never call back into the engine.
pub trait QueueStore: Send + Sync {
    /// The second value says whether the stored queue can be trusted (intact or not
    /// there yet). The engine only cleans up orphan packages when it is true.
    fn load(&self) -> (QueueState, bool);
    fn save(&self, state: &QueueState) -> io::Result<()>;
}

/// The pre-SQLite `queue.json` store. Kept for tests and the one-time import.
pub struct JsonQueueStore {
    pub path: PathBuf,
}

impl QueueStore for JsonQueueStore {
    fn load(&self) -> (QueueState, bool) {
        QueueState::load_checked(&self.path)
    }

    fn save(&self, state: &QueueState) -> io::Result<()> {
        state.save(&self.path)
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct QueueState {
    pub next_id: u64,
    pub jobs: Vec<Job>,
}

/// v0.1.x queue.json: jobs have `url` instead of `source`. Read them as YouTube jobs.
fn migrate_legacy(v: &mut serde_json::Value) {
    let Some(jobs) = v.get_mut("jobs").and_then(|j| j.as_array_mut()) else {
        return;
    };
    for job in jobs.iter_mut().filter_map(|j| j.as_object_mut()) {
        if !job.contains_key("source")
            && let Some(url) = job.remove("url")
        {
            job.insert(
                "source".into(),
                serde_json::json!({"kind": "youtube", "url": url}),
            );
        }
    }
}

impl QueueState {
    pub fn add(&mut self, new: NewJob, now_ms: u64) -> Job {
        let id = self.next_id.max(1);
        self.next_id = id + 1;
        let job = Job {
            id,
            source: new.source,
            title: new.title,
            thumbnail: new.thumbnail,
            duration_secs: new.duration_secs,
            quality_label: new.quality_label,
            options: new.options,
            state: JobState::Queued,
            progress: 0.0,
            speed: None,
            eta: None,
            output: None,
            work_dir: None,
            original: None,
            created_at: now_ms,
        };
        self.jobs.push(job.clone());
        job
    }

    pub fn get_mut(&mut self, id: u64) -> Option<&mut Job> {
        self.jobs.iter_mut().find(|j| j.id == id)
    }

    pub fn remove(&mut self, id: u64) -> Option<Job> {
        let i = self.jobs.iter().position(|j| j.id == id)?;
        Some(self.jobs.remove(i))
    }

    /// The oldest queued job that is not busy(id) (ids follow insertion order).
    pub fn next_queued(&self, busy: impl Fn(u64) -> bool) -> Option<u64> {
        self.jobs
            .iter()
            .filter(|j| j.state == JobState::Queued && !busy(j.id))
            .map(|j| j.id)
            .min()
    }

    /// The app quit mid-job: put running jobs back in the queue.
    pub fn recover(&mut self) {
        for j in &mut self.jobs {
            if j.state.is_active() {
                j.state = JobState::Queued;
                j.progress = 0.0;
                j.speed = None;
                j.eta = None;
            }
        }
    }

    pub fn load(path: &Path) -> QueueState {
        Self::load_checked(path).0
    }

    /// The second value says whether the file is trusted (valid or not there yet). False on corruption or read errors.
    pub fn load_checked(path: &Path) -> (QueueState, bool) {
        let (mut q, trusted): (QueueState, bool) = match fs::read_to_string(path) {
            Ok(text) => match serde_json::from_str::<serde_json::Value>(&text).and_then(|mut v| {
                migrate_legacy(&mut v);
                serde_json::from_value(v)
            }) {
                Ok(q) => (q, true),
                Err(e) => {
                    // Keep the corrupt file before the next save overwrites it.
                    log::error!("queue parse error, moved to .bad, starting empty: {e}");
                    let _ = fs::rename(path, path.with_extension("json.bad"));
                    (QueueState::default(), false)
                }
            },
            Err(e) => {
                let missing = e.kind() == io::ErrorKind::NotFound;
                if !missing {
                    log::error!("queue read error, starting empty: {e}");
                }
                (QueueState::default(), missing)
            }
        };
        q.repair_next_id();
        (q, trusted)
    }

    /// Repairs a corrupt next_id. An empty list keeps 0 and add counts from 1.
    pub fn repair_next_id(&mut self) {
        let min_next = self.jobs.iter().map(|j| j.id + 1).max().unwrap_or(0);
        self.next_id = self.next_id.max(min_next);
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        fs::write(
            &tmp,
            serde_json::to_string_pretty(self).map_err(io::Error::other)?,
        )?;
        fs::rename(tmp, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::JobSource;
    use crate::model::{JobOptions, Preset};

    #[test]
    fn loads_legacy_url_jobs() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("queue.json");
        // v0.1.x format: url without source, no max_height in options
        fs::write(
            &path,
            r#"{"next_id":2,"jobs":[{"id":1,"url":"https://youtu.be/x","title":"t",
               "options":{"format_id":null,"preset":"original","subtitles":[],"auto_subtitles":false},
               "state":{"kind":"completed"}}]}"#,
        )
        .unwrap();
        let (q, trusted) = QueueState::load_checked(&path);
        assert!(trusted);
        assert_eq!(q.jobs.len(), 1);
        assert_eq!(
            q.jobs[0].source,
            JobSource::Youtube {
                url: "https://youtu.be/x".into()
            }
        );
        assert_eq!(q.jobs[0].options.max_height, None);
        assert!(!path.with_extension("json.bad").exists());
    }

    fn new_job(title: &str) -> NewJob {
        NewJob {
            source: JobSource::Youtube {
                url: "https://youtu.be/x".into(),
            },
            title: title.into(),
            thumbnail: None,
            duration_secs: Some(10.0),
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

    #[test]
    fn ids_start_at_one_and_are_never_reused() {
        let mut q = QueueState::default();
        assert_eq!(q.add(new_job("a"), 10).id, 1);
        assert_eq!(q.add(new_job("b"), 11).id, 2);
        q.remove(2).unwrap();
        assert_eq!(q.add(new_job("c"), 12).id, 3);
        let j = &q.jobs[0];
        assert_eq!(
            (j.state.clone(), j.progress, j.created_at),
            (JobState::Queued, 0.0, 10)
        );
    }

    #[test]
    fn next_queued_is_oldest_not_busy() {
        let mut q = QueueState::default();
        for t in ["a", "b", "c"] {
            q.add(new_job(t), 0);
        }
        q.get_mut(1).unwrap().state = JobState::Completed;
        assert_eq!(q.next_queued(|_| false), Some(2));
        assert_eq!(q.next_queued(|id| id == 2), Some(3));
        assert_eq!(q.next_queued(|_| true), None);
    }

    #[test]
    fn recover_requeues_active_jobs() {
        let mut q = QueueState::default();
        for t in ["a", "b", "c"] {
            q.add(new_job(t), 0);
        }
        q.get_mut(1).unwrap().state = JobState::Downloading;
        q.get_mut(1).unwrap().progress = 0.4;
        q.get_mut(2).unwrap().state = JobState::Encoding;
        q.get_mut(3).unwrap().state = JobState::Stopped;
        q.recover();
        assert_eq!(q.jobs[0].state, JobState::Queued);
        assert_eq!(q.jobs[0].progress, 0.0);
        assert_eq!(q.jobs[1].state, JobState::Queued);
        assert_eq!(q.jobs[2].state, JobState::Stopped);
    }

    #[test]
    fn save_load_roundtrip() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("data/queue.json");
        let mut q = QueueState::default();
        q.add(new_job("a"), 5);
        q.save(&path).unwrap();
        assert_eq!(QueueState::load(&path), q);
    }

    #[test]
    fn load_missing_or_corrupt_is_empty() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(
            QueueState::load(&d.path().join("none.json")),
            QueueState::default()
        );
        let bad = d.path().join("bad.json");
        fs::write(&bad, "{nope").unwrap();
        assert_eq!(
            QueueState::load_checked(&bad),
            (QueueState::default(), false)
        );
        assert!(!bad.exists());
        assert_eq!(
            fs::read_to_string(d.path().join("bad.json.bad")).unwrap(),
            "{nope"
        );
    }

    #[test]
    fn load_tolerates_missing_optional_fields() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("queue.json");
        fs::write(
            &path,
            r#"{"next_id":2,"jobs":[{"id":1,"url":"https://youtu.be/x","title":"a",
            "options":{"format_id":null,"preset":"mp3","subtitles":[],"auto_subtitles":false},
            "state":{"kind":"queued"}}]}"#,
        )
        .unwrap();
        let q = QueueState::load(&path);
        assert_eq!(q.jobs.len(), 1);
        assert_eq!((q.jobs[0].progress, q.jobs[0].output.clone()), (0.0, None));
    }

    #[test]
    fn load_repairs_next_id() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("queue.json");
        let mut q = QueueState::default();
        q.add(new_job("a"), 0);
        q.add(new_job("b"), 0);
        q.next_id = 1; // corrupt file
        q.save(&path).unwrap();
        let mut loaded = QueueState::load(&path);
        assert_eq!(loaded.add(new_job("c"), 0).id, 3);
    }
}
