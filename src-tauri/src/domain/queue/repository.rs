//! SQL for the queue: one `jobs` row per job (`data` is its JSON) plus `meta.next_job_id`.

use super::model::QueueState;
use crate::shared::error::{AppError, AppResult};
use kiri_core::queue::QueueStore;
use rusqlite::{Connection, OptionalExtension, types::Value};
use std::{
    io,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

const NEXT_ID: &str = "next_job_id";

pub fn is_empty(conn: &Connection) -> AppResult<bool> {
    Ok(!conn.query_row("SELECT EXISTS (SELECT 1 FROM jobs)", [], |r| r.get(0))?)
}

/// The stored queue, oldest job first. A row whose `data` is not job JSON text is moved
/// to `meta.corrupt_job:<id>` so later saves cannot erase it, and makes the queue untrusted.
pub fn load(conn: &Connection) -> AppResult<(QueueState, bool)> {
    let rows: Vec<(i64, Value)> = conn
        .prepare("SELECT id, data FROM jobs ORDER BY id")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let mut q = QueueState::default();
    let mut trusted = true;
    for (id, data) in rows {
        let parsed = match &data {
            Value::Text(text) => serde_json::from_str(text).map_err(|e| e.to_string()),
            other => Err(format!("data is {:?}, not text", other.data_type())),
        };
        match parsed {
            Ok(job) => q.jobs.push(job),
            Err(e) => {
                log::error!("queue row {id} is not a valid job, moved to meta: {e}");
                let tx = conn.unchecked_transaction()?;
                tx.execute(
                    "INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)",
                    (format!("corrupt_job:{id}"), &data),
                )?;
                tx.execute("DELETE FROM jobs WHERE id = ?1", [id])?;
                tx.commit()?;
                trusted = false;
            }
        }
    }
    q.next_id = conn
        .query_row("SELECT value FROM meta WHERE key = ?1", [NEXT_ID], |r| {
            r.get::<_, String>(0)
        })
        .optional()?
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    q.repair_next_id();
    Ok((q, trusted))
}

/// Replaces every job row and `next_job_id` in one transaction.
// ponytail: rewrites the whole table per save; fine for a queue of tens of jobs,
// switch to per-job upserts if queues grow to thousands.
pub fn save(conn: &Connection, q: &QueueState) -> AppResult<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute("DELETE FROM jobs", [])?;
    {
        let mut insert = tx.prepare("INSERT INTO jobs (id, data) VALUES (?1, ?2)")?;
        for job in &q.jobs {
            let data = serde_json::to_string(job).map_err(|e| AppError::Failed(e.to_string()))?;
            insert.execute((job.id as i64, data))?;
        }
    }
    tx.execute(
        "INSERT INTO meta (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        (NEXT_ID, q.next_id.to_string()),
    )?;
    tx.commit()?;
    Ok(())
}

/// The engine's store. It locks the DB only inside `load`/`save`; the engine calls
/// `save` while holding its state lock, so code holding the DB lock must never call
/// into the engine.
pub struct SqliteQueueStore {
    db: Arc<Mutex<Connection>>,
    /// False when the legacy `queue.json` import failed. The load is then untrusted,
    /// as the JSON store's was for a corrupt file.
    import_ok: bool,
    /// Set when `load` fails. Saves are then refused: the engine's empty queue would
    /// otherwise replace job rows that were never read.
    read_failed: AtomicBool,
}

impl SqliteQueueStore {
    pub fn new(db: Arc<Mutex<Connection>>, import_ok: bool) -> Self {
        Self {
            db,
            import_ok,
            read_failed: AtomicBool::new(false),
        }
    }
}

impl QueueStore for SqliteQueueStore {
    // ponytail: a failed read starts empty and every save is refused until restart, so
    // the session's queue changes are not persisted; add a retrying load if reads fail
    // transiently in practice.
    fn load(&self) -> (QueueState, bool) {
        match load(&self.db.lock().unwrap()) {
            Ok((q, trusted)) => (q, trusted && self.import_ok),
            Err(e) => {
                log::error!("cannot read the queue, starting empty and not saving: {e}");
                self.read_failed.store(true, Ordering::Relaxed);
                (QueueState::default(), false)
            }
        }
    }

    fn save(&self, state: &QueueState) -> io::Result<()> {
        if self.read_failed.load(Ordering::Relaxed) {
            return Err(io::Error::other("queue read failed; not overwriting"));
        }
        save(&self.db.lock().unwrap(), state).map_err(io::Error::other)
    }
}
