//! SQLite connection: opening the app database and running schema migrations.

use crate::shared::error::{AppError, AppResult};
use rusqlite::Connection;
use std::path::Path;

/// One entry per schema version; `PRAGMA user_version` records how many have run.
/// Never edit or reorder a shipped entry; append a new file instead.
const MIGRATIONS: &[&str] = &[include_str!("migrations/0001_init.sql")];

/// Opens (creating if needed) the database at `path` and brings it to the latest schema.
pub fn open(path: &Path) -> AppResult<Connection> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut conn = Connection::open(path)?;
    migrate(&mut conn)?;
    Ok(conn)
}

/// Runs every pending migration, each in its own transaction together with its
/// `user_version` bump. Refuses a DB written by a newer build.
pub fn migrate(conn: &mut Connection) -> AppResult<()> {
    let supported = MIGRATIONS.len() as i64;
    let found: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if found > supported {
        return Err(AppError::SchemaTooNew { found, supported });
    }
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(found as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", i as i64 + 1)?;
        tx.commit()?;
        log::info!("applied migration {}", i + 1);
    }
    Ok(())
}

/// Migrated in-memory DB for tests.
#[cfg(test)]
pub fn open_in_memory() -> Connection {
    let mut conn = Connection::open_in_memory().unwrap();
    migrate(&mut conn).unwrap();
    conn
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::error::AppError;

    fn user_version(conn: &Connection) -> i64 {
        conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap()
    }

    #[test]
    fn fresh_db_is_at_latest_version() {
        let conn = open_in_memory();
        assert_eq!(user_version(&conn), MIGRATIONS.len() as i64);
    }

    #[test]
    fn reopen_does_not_rerun() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/kiri.db");
        let conn = open(&path).unwrap();
        conn.execute("INSERT INTO meta (key, value) VALUES ('next_job_id', '7')", [])
            .unwrap();
        drop(conn);

        let conn = open(&path).unwrap();
        let v: String = conn
            .query_row("SELECT value FROM meta WHERE key = 'next_job_id'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, "7");
        assert_eq!(user_version(&conn), MIGRATIONS.len() as i64);
    }

    #[test]
    fn newer_schema_is_refused() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "user_version", 99).unwrap();
        let err = migrate(&mut conn).unwrap_err();
        assert!(matches!(err, AppError::SchemaTooNew { found: 99, supported: 1 }));
    }
}
