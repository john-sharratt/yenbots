//! The store: one SQLite file, compiled in, its schema brought up to date by
//! the migrations embedded in the binary ([`migrations`]) every time it opens
//! (`candle/docs/yenbot_app_host.md` §7).
//!
//! One connection, behind a mutex: the site's writes are a family's, and
//! SQLite in WAL mode serves them from one writer with room to spare.

pub mod backups;
pub mod migrations;

use std::path::Path;
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use anyhow::{Context, Result};
use rusqlite::Connection;

pub struct Store {
    conn: Mutex<Connection>,
}

impl Store {
    /// The store at `path`, created when absent, migrated to this build's
    /// schema.
    pub fn open(path: &Path) -> Result<Store> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).with_context(|| format!("making {}", dir.display()))?;
        }
        let conn = Connection::open(path).with_context(|| format!("opening {}", path.display()))?;
        Store::on(conn)
    }

    /// A store held in memory, for tests.
    #[cfg(test)]
    pub fn in_memory() -> Result<Store> {
        Store::on(Connection::open_in_memory()?)
    }

    fn on(mut conn: Connection) -> Result<Store> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.busy_timeout(Duration::from_secs(5))?;
        migrations::migrate(&mut conn)?;
        Ok(Store {
            conn: Mutex::new(conn),
        })
    }

    /// The connection, for one unit of work.
    pub fn conn(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The schema version the store is at.
    pub fn version(&self) -> Result<u32> {
        migrations::version(&self.conn())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_store_is_at_this_builds_schema_and_reopens_there() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data").join("yenbot.db");
        let store = Store::open(&path).unwrap();
        assert_eq!(store.version().unwrap(), migrations::LATEST);
        drop(store);
        assert_eq!(
            Store::open(&path).unwrap().version().unwrap(),
            migrations::LATEST
        );
    }
}
