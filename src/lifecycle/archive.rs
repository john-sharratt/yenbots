//! Apps archived (§20.1). Deleting an app is removing its folder: the site
//! sees an app with records, files or jobs and no folder, and archives it —
//! not served, its waiting jobs held, everything of it kept thirty days and
//! listed on the admin page — then a platform job purges it. Its folder
//! coming back restores it. An admin may also archive an app whose folder
//! stays: it is hidden until restored, and kept for as long as its folder
//! is there.

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use serde_json::Value;

use super::tables::APP_TABLES;
use crate::jobs::table::PLATFORM;

/// How long an app's things are kept once its folder is gone.
pub const KEPT_MS: i64 = 30 * 86_400_000;

/// Why an app is archived.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    /// Its folder was removed.
    Removed,
    /// An admin archived it.
    Admin,
}

impl Reason {
    fn as_str(self) -> &'static str {
        match self {
            Reason::Removed => "removed",
            Reason::Admin => "admin",
        }
    }

    fn parse(s: &str) -> Reason {
        if s == "admin" {
            Reason::Admin
        } else {
            Reason::Removed
        }
    }
}

/// An archived app, as the admin page lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Archived {
    pub app: String,
    pub reason: Reason,
    pub archived_ms: i64,
    /// When it is purged; `None` while its folder is there.
    pub purge_ms: Option<i64>,
    pub records: i64,
    pub files: i64,
}

/// The apps archived, by id.
pub fn ids(conn: &Connection) -> Result<BTreeSet<String>> {
    let mut stmt = conn.prepare("SELECT app FROM archived_apps")?;
    let rows = stmt.query_map([], |r| r.get(0))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Why `app` is archived, when it is.
pub fn reason_of(conn: &Connection, app: &str) -> Result<Option<Reason>> {
    let held: Option<String> = conn
        .query_row(
            "SELECT reason FROM archived_apps WHERE app = ?1",
            params![app],
            |r| r.get(0),
        )
        .optional()?;
    Ok(held.as_deref().map(Reason::parse))
}

/// The apps that hold records, files or jobs — the platform's own aside.
pub fn with_data(conn: &Connection) -> Result<BTreeSet<String>> {
    let mut stmt = conn
        .prepare("SELECT app FROM docs UNION SELECT app FROM files UNION SELECT app FROM jobs")?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    let mut out: BTreeSet<String> = rows.collect::<Result<_, _>>()?;
    out.remove(PLATFORM);
    Ok(out)
}

/// Archive `app` for `reason`, holding its waiting jobs; whether it was not
/// archived already.
pub fn archive(conn: &Connection, app: &str, reason: Reason, now: i64) -> Result<bool> {
    if reason_of(conn, app)?.is_some() {
        return Ok(false);
    }
    let mut stmt = conn.prepare("SELECT id FROM jobs WHERE app = ?1 AND state = 'waiting'")?;
    let held: Vec<String> = stmt
        .query_map(params![app], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    conn.execute(
        "UPDATE jobs SET state = 'held', updated_ms = ?2 WHERE app = ?1 AND state = 'waiting'",
        params![app, now],
    )?;
    conn.execute(
        "INSERT INTO archived_apps (app, reason, held_jobs, archived_ms, purge_ms) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            app,
            reason.as_str(),
            Value::from(held).to_string(),
            now,
            now + KEPT_MS
        ],
    )?;
    Ok(true)
}

/// `app`'s folder is gone: an app an admin archived is now kept thirty days
/// from `now`, like any removed one.
pub fn folder_gone(conn: &Connection, app: &str, now: i64) -> Result<()> {
    conn.execute(
        "UPDATE archived_apps SET reason = 'removed', purge_ms = ?2 WHERE app = ?1 AND reason = 'admin'",
        params![app, now + KEPT_MS],
    )?;
    Ok(())
}

/// Restore `app`: the jobs archiving held wait again; whether it was
/// archived.
pub fn restore(conn: &Connection, app: &str, now: i64) -> Result<bool> {
    let held: Option<String> = conn
        .query_row(
            "SELECT held_jobs FROM archived_apps WHERE app = ?1",
            params![app],
            |r| r.get(0),
        )
        .optional()?;
    let Some(held) = held else {
        return Ok(false);
    };
    let ids: Vec<String> = serde_json::from_str(&held).unwrap_or_default();
    for id in ids {
        conn.execute(
            "UPDATE jobs SET state = 'waiting', updated_ms = ?3 WHERE app = ?1 AND id = ?2 AND state = 'held'",
            params![app, id, now],
        )?;
    }
    conn.execute("DELETE FROM archived_apps WHERE app = ?1", params![app])?;
    Ok(true)
}

/// The archived apps, oldest first.
pub fn list(conn: &Connection) -> Result<Vec<Archived>> {
    let mut stmt = conn.prepare(
        "SELECT a.app, a.reason, a.archived_ms, a.purge_ms, \
           (SELECT COUNT(*) FROM docs d WHERE d.app = a.app), \
           (SELECT COUNT(*) FROM files f WHERE f.app = a.app) \
         FROM archived_apps a ORDER BY a.archived_ms, a.app",
    )?;
    let rows = stmt.query_map([], |r| {
        let reason = Reason::parse(&r.get::<_, String>(1)?);
        let purge: i64 = r.get(3)?;
        Ok(Archived {
            app: r.get(0)?,
            reason,
            archived_ms: r.get(2)?,
            purge_ms: (reason == Reason::Removed).then_some(purge),
            records: r.get(4)?,
            files: r.get(5)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// The removed apps whose thirty days are over at `now`.
pub fn due(conn: &Connection, now: i64) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT app FROM archived_apps WHERE reason = 'removed' AND purge_ms <= ?1 ORDER BY app",
    )?;
    let rows = stmt.query_map(params![now], |r| r.get(0))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Remove everything of `app` — its rows in every table and its files under
/// `files_dir` — and its place in the archive.
pub fn purge(conn: &mut Connection, files_dir: &Path, app: &str) -> Result<()> {
    let tx = conn.transaction()?;
    for table in APP_TABLES {
        tx.execute(&format!("DELETE FROM {table} WHERE app = ?1"), params![app])?;
    }
    tx.execute("DELETE FROM archived_apps WHERE app = ?1", params![app])?;
    tx.commit()?;
    let dir = files_dir.join(app);
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;

    fn seeded() -> Store {
        let store = Store::in_memory().unwrap();
        store
            .conn()
            .execute_batch(
                "INSERT INTO docs (app, coll, id, owner, data, version, created_ms, updated_ms) \
                   VALUES ('chores', 'chores', 'c1', '1', '{}', 1, 0, 0); \
                 INSERT INTO files (app, id, owner, name, kind, bytes, created_ms) \
                   VALUES ('chores', 'f1', '1', 'x.png', 'png', 3, 0); \
                 INSERT INTO jobs (id, app, workflow, args, source, state, updated_ms) \
                   VALUES ('j1', 'chores', 'remind', '{}', 'schedule:0', 'waiting', 0), \
                          ('j2', 'chores', 'other', '{}', 'schedule:1', 'held', 0), \
                          ('p', '_platform', 'backup', '{}', 'schedule:backup', 'waiting', 0);",
            )
            .unwrap();
        store
    }

    fn state(store: &Store, id: &str) -> String {
        store
            .conn()
            .query_row("SELECT state FROM jobs WHERE id = ?1", [id], |r| r.get(0))
            .unwrap()
    }

    /// **An archived app's waiting jobs are held, and restoring it lets them
    /// wait again** — a job held before it was archived stays held.
    #[test]
    fn archiving_holds_jobs_and_restoring_releases_them() {
        let store = seeded();
        let conn = store.conn();
        assert_eq!(
            with_data(&conn).unwrap(),
            BTreeSet::from(["chores".to_string()])
        );
        assert!(archive(&conn, "chores", Reason::Removed, 10).unwrap());
        assert!(
            !archive(&conn, "chores", Reason::Admin, 11).unwrap(),
            "once"
        );
        drop(conn);
        assert_eq!(
            (state(&store, "j1"), state(&store, "j2")),
            ("held".into(), "held".into())
        );
        let conn = store.conn();
        assert_eq!(
            list(&conn).unwrap(),
            [Archived {
                app: "chores".into(),
                reason: Reason::Removed,
                archived_ms: 10,
                purge_ms: Some(10 + KEPT_MS),
                records: 1,
                files: 1,
            }]
        );
        assert_eq!(due(&conn, 10 + KEPT_MS - 1).unwrap(), Vec::<String>::new());
        assert_eq!(due(&conn, 10 + KEPT_MS).unwrap(), ["chores"]);
        assert!(restore(&conn, "chores", 20).unwrap());
        assert!(!restore(&conn, "chores", 21).unwrap());
        drop(conn);
        assert_eq!(
            (state(&store, "j1"), state(&store, "j2")),
            ("waiting".into(), "held".into())
        );
    }

    /// **An app an admin archived is kept while its folder is there**, and
    /// thirty days once it is gone.
    #[test]
    fn an_admins_archive_is_kept_while_its_folder_is() {
        let store = seeded();
        let conn = store.conn();
        archive(&conn, "chores", Reason::Admin, 10).unwrap();
        assert_eq!(list(&conn).unwrap()[0].purge_ms, None);
        assert!(due(&conn, i64::MAX).unwrap().is_empty());
        folder_gone(&conn, "chores", 50).unwrap();
        assert_eq!(list(&conn).unwrap()[0].purge_ms, Some(50 + KEPT_MS));
    }

    /// **A purge leaves nothing of the app**, and the platform's own alone.
    #[test]
    fn a_purge_leaves_nothing_of_the_app() {
        let store = seeded();
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("chores")).unwrap();
        std::fs::write(dir.path().join("chores").join("f1"), b"png").unwrap();
        archive(&store.conn(), "chores", Reason::Removed, 0).unwrap();
        purge(&mut store.conn(), dir.path(), "chores").unwrap();
        let conn = store.conn();
        for table in APP_TABLES {
            let n: i64 = conn
                .query_row(
                    &format!("SELECT COUNT(*) FROM {table} WHERE app = 'chores'"),
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(n, 0, "{table}");
        }
        assert!(ids(&conn).unwrap().is_empty());
        assert!(!dir.path().join("chores").exists());
        drop(conn);
        assert_eq!(state(&store, "p"), "waiting");
    }
}
