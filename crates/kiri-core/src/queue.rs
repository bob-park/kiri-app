//! 큐의 순수 상태. 동시성·프로세스는 engine 이 맡고, 여기는 목록과 파일만 다룬다.
use crate::model::{Job, JobState, NewJob};
use serde::{Deserialize, Serialize};
use std::{fs, io, path::Path};

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct QueueState {
    pub next_id: u64,
    pub jobs: Vec<Job>,
}

impl QueueState {
    pub fn add(&mut self, new: NewJob, now_ms: u64) -> Job {
        let id = self.next_id.max(1);
        self.next_id = id + 1;
        let job = Job {
            id,
            url: new.url,
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

    /// busy(id) 가 아닌 대기 작업 중 가장 오래된 것 (id 가 곧 추가 순서).
    pub fn next_queued(&self, busy: impl Fn(u64) -> bool) -> Option<u64> {
        self.jobs
            .iter()
            .filter(|j| j.state == JobState::Queued && !busy(j.id))
            .map(|j| j.id)
            .min()
    }

    /// 앱이 작업 도중 꺼졌던 경우: 돌던 작업을 다시 대기열로.
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
        let mut q: QueueState = match fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
                eprintln!("kiri queue: parse error, starting empty: {e}");
                QueueState::default()
            }),
            Err(e) => {
                if e.kind() != io::ErrorKind::NotFound {
                    eprintln!("kiri queue: read error, starting empty: {e}");
                }
                QueueState::default()
            }
        };
        // 손상된 next_id 보정. 빈 목록이면 0 그대로 두고 add 가 1 부터 센다.
        let min_next = q.jobs.iter().map(|j| j.id + 1).max().unwrap_or(0);
        q.next_id = q.next_id.max(min_next);
        q
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
    use crate::model::{JobOptions, Preset};

    fn new_job(title: &str) -> NewJob {
        NewJob {
            url: "https://youtu.be/x".into(),
            title: title.into(),
            thumbnail: None,
            duration_secs: Some(10.0),
            quality_label: "720p".into(),
            options: JobOptions {
                format_id: Some("136".into()),
                preset: Preset::Original,
                subtitles: vec![],
                auto_subtitles: false,
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
        assert_eq!(QueueState::load(&bad), QueueState::default());
    }

    #[test]
    fn load_repairs_next_id() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("queue.json");
        let mut q = QueueState::default();
        q.add(new_job("a"), 0);
        q.add(new_job("b"), 0);
        q.next_id = 1; // 손상된 파일
        q.save(&path).unwrap();
        let mut loaded = QueueState::load(&path);
        assert_eq!(loaded.add(new_job("c"), 0).id, 3);
    }
}
