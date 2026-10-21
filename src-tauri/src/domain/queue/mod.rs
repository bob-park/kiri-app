//! Queue domain: persists the engine's job list in SQLite and imports the legacy `queue.json`.

pub mod commands;
mod model;
pub mod repository;

pub use model::{Job, NewJob};
pub use repository::SqliteQueueStore;

use crate::shared::error::{AppError, AppResult};
use kiri_core::queue::{JsonQueueStore, QueueStore};
use rusqlite::Connection;
use std::{fs, path::Path};

/// One-time import of the legacy `queue.json` (v0.1 jobs included): only when the
/// `jobs` table is empty and the file exists. The file is renamed to
/// `queue.json.migrated`. Returns whether it ran. A corrupt file is kept as
/// `queue.json.bad` and an unreadable one stays in place; both save nothing and
/// return `Failed`, so the caller can treat the queue as untrusted.
pub fn import_json(conn: &Connection, path: &Path) -> AppResult<bool> {
    if !repository::is_empty(conn)? || !path.exists() {
        return Ok(false);
    }
    let (q, trusted) = JsonQueueStore { path: path.into() }.load();
    if !trusted {
        return Err(AppError::Failed(format!(
            "{} is corrupt or unreadable, nothing imported",
            path.display()
        )));
    }
    repository::save(conn, &q)?;
    fs::rename(path, path.with_extension("json.migrated"))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::{model::QueueState, *};
    use crate::shared::{db::open_in_memory, error::AppError};
    use kiri_core::{
        model::{JobOptions, JobSource, JobState, Preset},
        queue::QueueStore,
    };
    use std::{
        fs,
        sync::{Arc, Mutex},
    };

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
                format_id: None,
                preset: Preset::Original,
                subtitles: vec![],
                auto_subtitles: false,
                max_height: None,
            },
        }
    }

    fn store(conn: rusqlite::Connection) -> SqliteQueueStore {
        SqliteQueueStore::new(Arc::new(Mutex::new(conn)), true)
    }

    #[test]
    fn roundtrip_preserves_jobs_and_next_id() {
        let s = store(open_in_memory());
        let mut q = QueueState::default();
        for t in ["a", "b", "c"] {
            q.add(new_job(t), 1);
        }
        q.remove(3); // next_id stays 4 even though the highest stored id is 2
        q.get_mut(1).unwrap().state = JobState::Completed;
        s.save(&q).unwrap();
        assert_eq!(s.load(), (q.clone(), true));

        q.remove(1);
        s.save(&q).unwrap();
        let (loaded, trusted) = s.load();
        assert!(trusted);
        assert_eq!(loaded.jobs.iter().map(|j| j.id).collect::<Vec<_>>(), [2]);
        assert_eq!(loaded.next_id, 4);
    }

    #[test]
    fn corrupt_row_is_untrusted() {
        let conn = open_in_memory();
        let mut q = QueueState::default();
        q.add(new_job("a"), 1);
        q.add(new_job("b"), 1);
        repository::save(&conn, &q).unwrap();
        conn.execute("INSERT INTO jobs (id, data) VALUES (9, '{')", [])
            .unwrap();
        let db = Arc::new(Mutex::new(conn));
        let s = SqliteQueueStore::new(db.clone(), true);
        let (loaded, trusted) = s.load();
        assert!(!trusted);
        assert_eq!(loaded.jobs, q.jobs);

        // the bad row survives the engine's next save
        s.save(&loaded).unwrap();
        let kept: String = db
            .lock()
            .unwrap()
            .query_row(
                "SELECT value FROM meta WHERE key = 'corrupt_job:9'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(kept, "{");
    }

    #[test]
    fn blob_row_is_quarantined_and_others_survive_save() {
        let conn = open_in_memory();
        let mut q = QueueState::default();
        q.add(new_job("a"), 1);
        q.add(new_job("b"), 1);
        repository::save(&conn, &q).unwrap();
        conn.execute("INSERT INTO jobs (id, data) VALUES (9, x'00ff')", [])
            .unwrap();
        let db = Arc::new(Mutex::new(conn));
        let s = SqliteQueueStore::new(db.clone(), true);
        let (loaded, trusted) = s.load();
        assert!(!trusted);
        assert_eq!(loaded.jobs, q.jobs);

        s.save(&loaded).unwrap();
        let conn = db.lock().unwrap();
        let kept: Vec<u8> = conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'corrupt_job:9'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(kept, [0x00, 0xff]);
        assert_eq!(repository::load(&conn).unwrap().0.jobs, q.jobs);
    }

    #[test]
    fn failed_load_refuses_saves() {
        let conn = open_in_memory();
        let mut q = QueueState::default();
        q.add(new_job("a"), 1);
        repository::save(&conn, &q).unwrap();
        conn.execute_batch("ALTER TABLE jobs RENAME TO jobs_hidden")
            .unwrap();
        let db = Arc::new(Mutex::new(conn));
        let s = SqliteQueueStore::new(db.clone(), true);
        assert_eq!(s.load(), (QueueState::default(), false));

        // the table comes back; the empty queue must not overwrite it
        db.lock()
            .unwrap()
            .execute_batch("ALTER TABLE jobs_hidden RENAME TO jobs")
            .unwrap();
        assert!(s.save(&QueueState::default()).is_err());
        assert_eq!(repository::load(&db.lock().unwrap()).unwrap().0, q);
    }

    #[test]
    fn failed_import_makes_first_load_untrusted() {
        let s = SqliteQueueStore::new(Arc::new(Mutex::new(open_in_memory())), false);
        assert_eq!(s.load(), (QueueState::default(), false));
    }

    #[test]
    fn import_json_requeues_active_job() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("queue.json");
        fs::write(
            &path,
            r#"{"next_id":5,"jobs":[{"id":4,"url":"https://youtu.be/x","title":"t",
               "options":{"format_id":null,"preset":"original","subtitles":[],"auto_subtitles":false},
               "state":{"kind":"downloading"},"progress":0.5}]}"#,
        )
        .unwrap();
        let conn = open_in_memory();
        assert!(import_json(&conn, &path).unwrap());
        assert!(!path.exists());
        assert!(d.path().join("queue.json.migrated").exists());

        let (mut q, trusted) = repository::load(&conn).unwrap();
        assert!(trusted);
        q.recover();
        assert_eq!(q.next_id, 5);
        assert_eq!(q.jobs.len(), 1);
        assert_eq!(
            (q.jobs[0].state.clone(), q.jobs[0].progress),
            (JobState::Queued, 0.0)
        );
        assert_eq!(
            q.jobs[0].source,
            JobSource::Youtube {
                url: "https://youtu.be/x".into()
            }
        );
        // runs once
        assert!(!import_json(&conn, &d.path().join("queue.json.migrated")).unwrap());
    }

    #[test]
    fn import_json_skips_when_db_has_jobs_or_file_missing() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("queue.json");
        let conn = open_in_memory();
        assert!(!import_json(&conn, &path).unwrap());

        let mut q = QueueState::default();
        q.add(new_job("db"), 1);
        repository::save(&conn, &q).unwrap();
        fs::write(&path, r#"{"next_id":1,"jobs":[]}"#).unwrap();
        assert!(!import_json(&conn, &path).unwrap());
        assert!(path.exists());
        assert_eq!(repository::load(&conn).unwrap().0, q);
    }

    #[test]
    fn import_corrupt_json_fails_and_keeps_the_file() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("queue.json");
        fs::write(&path, "{nope").unwrap();
        let conn = open_in_memory();
        assert!(matches!(
            import_json(&conn, &path),
            Err(AppError::Failed(_))
        ));
        assert!(repository::is_empty(&conn).unwrap());
        assert_eq!(
            fs::read_to_string(d.path().join("queue.json.bad")).unwrap(),
            "{nope"
        );
    }
}
