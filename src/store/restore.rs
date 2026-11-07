//! Restoring from a backup (`candle/docs/yenbot_app_host.md` §7.4): one app's
//! records — all its collections, or those named — in one transaction, the
//! rest of the site untouched; or the whole store, copied in under the
//! store's lock. Either first backs up what it replaces, as
//! `replaced-<ms>.db` beside the nightly backups.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use rusqlite::backup::Progress;
use rusqlite::{params, Connection, DatabaseName, OpenFlags};

use super::migrations::{migrate, version, LATEST};
use super::Store;

/// The first schema step that holds the apps' records.
const RECORDS_FROM: u32 = 2;

/// The first schema step that records each record's shape.
const SHAPES_FROM: u32 = 3;

/// The first schema step that keeps the apps' files.
const FILES_FROM: u32 = 5;

/// Back up the store as it is into `dir`, before a restore replaces it.
fn keep_replaced(conn: &Connection, dir: &Path, now: i64) -> Result<PathBuf> {
    std::fs::create_dir_all(dir).with_context(|| format!("making {}", dir.display()))?;
    let file = dir.join(format!("replaced-{now}.db"));
    conn.execute("VACUUM INTO ?1", [file.to_string_lossy().to_string()])
        .with_context(|| format!("writing {}", file.display()))?;
    Ok(file)
}

/// What a restore did: how many records it put back, and where what it
/// replaced was kept.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Restored {
    pub records: u64,
    pub replaced: PathBuf,
    /// The ids of the app's files the backup knows of, whose bytes the
    /// backups' copy brings back (`crate::files::mirror`); none for a whole
    /// store, whose files were never taken away.
    pub files: Vec<String>,
}

/// Replace app `app`'s records — in `collections`, or all of them — and
/// their shares and search rows with the backup `from`'s.
pub fn app(
    store: &Store,
    from: &Path,
    app: &str,
    collections: Option<&[String]>,
    dir: &Path,
    now: i64,
) -> Result<Restored> {
    let mut conn = store.conn();
    let replaced = keep_replaced(&conn, dir, now)?;
    let path = from.to_string_lossy().to_string();
    conn.execute("ATTACH DATABASE ?1 AS b", [path])
        .with_context(|| format!("opening {}", from.display()))?;
    let done = put_back(&mut conn, app, collections);
    conn.execute_batch("DETACH DATABASE b")?;
    let (records, files) = done?;
    Ok(Restored {
        records,
        replaced,
        files,
    })
}

/// Put the records back; answers how many, and the ids of the app's files
/// the backup knows of.
fn put_back(
    conn: &mut Connection,
    app: &str,
    collections: Option<&[String]>,
) -> Result<(u64, Vec<String>)> {
    let held = "SELECT MAX(version) FROM b.schema_version";
    let at: Option<u32> = conn
        .query_row(held, [], |r| r.get(0))
        .context("the backup has no schema version")?;
    let at = at.unwrap_or(0);
    if !(RECORDS_FROM..=LATEST).contains(&at) {
        bail!("the backup is at schema {at}; this build restores records from schemas {RECORDS_FROM} to {LATEST}");
    }
    let named = collections.map(|c| serde_json::to_string(c).expect("strings serialize"));
    let only = "app = ?1 AND (?2 IS NULL OR coll IN (SELECT value FROM json_each(?2)))";
    let tx = conn.transaction()?;
    for table in ["docs_fts", "docs", "shares"] {
        let sql = format!("DELETE FROM main.{table} WHERE {only}");
        tx.execute(&sql, params![app, named])?;
    }
    // A backup from before records kept their shape holds them all at the
    // first, which the column's default gives them; the app's migrations
    // then take them up to its own (`crate::reshape`).
    let columns = if at >= SHAPES_FROM {
        "app, coll, id, owner, data, version, created_ms, updated_ms, shape"
    } else {
        "app, coll, id, owner, data, version, created_ms, updated_ms"
    };
    let records = tx.execute(
        &format!("INSERT INTO main.docs ({columns}) SELECT {columns} FROM b.docs WHERE {only}"),
        params![app, named],
    )?;
    tx.execute(
        &format!(
            "INSERT INTO main.docs_fts (app, coll, id, body) \
             SELECT app, coll, id, body FROM b.docs_fts WHERE {only}"
        ),
        params![app, named],
    )?;
    tx.execute(
        &format!("INSERT INTO main.shares SELECT * FROM b.shares WHERE {only}"),
        params![app, named],
    )?;
    // A file never changes: those the backup knows of come back beside any
    // kept since, for the records that name them.
    let mut files = Vec::new();
    if at >= FILES_FROM {
        tx.execute(
            "INSERT OR IGNORE INTO main.files SELECT * FROM b.files WHERE app = ?1",
            params![app],
        )?;
        let mut stmt = tx.prepare("SELECT id FROM b.files WHERE app = ?1 ORDER BY id")?;
        let rows = stmt.query_map(params![app], |r| r.get(0))?;
        files = rows.collect::<Result<_, _>>()?;
    }
    tx.commit()?;
    Ok((records as u64, files))
}

/// Replace the whole store with the backup `from`, brought up to this
/// build's schema.
pub fn whole(store: &Store, from: &Path, dir: &Path, now: i64) -> Result<Restored> {
    let backup = Connection::open_with_flags(from, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let at = version(&backup).with_context(|| format!("reading {}", from.display()))?;
    drop(backup);
    if at > LATEST {
        bail!("the backup is at schema {at}, newer than this build's {LATEST}");
    }
    let mut conn = store.conn();
    let replaced = keep_replaced(&conn, dir, now)?;
    conn.restore(DatabaseName::Main, from, None::<fn(Progress)>)
        .with_context(|| format!("copying in {}", from.display()))?;
    migrate(&mut conn)?;
    let records: u64 = conn.query_row("SELECT COUNT(*) FROM docs", [], |r| r.get(0))?;
    tracing::warn!(from = %from.display(), replaced = %replaced.display(), "store: restored whole");
    Ok(Restored {
        records,
        replaced,
        files: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use jiff::tz::TimeZone;

    use super::*;
    use crate::store::backups;
    use crate::store::migrations::STEPS;

    fn back_up(store: &Store, dir: &Path, at_ms: i64) -> Result<Option<PathBuf>> {
        backups::back_up(store, dir, at_ms, &TimeZone::UTC)
    }

    fn file_for(dir: &Path, at_ms: i64) -> PathBuf {
        backups::file_for(dir, at_ms, &TimeZone::UTC)
    }

    fn put(store: &Store, app: &str, coll: &str, id: &str, body: &str) {
        let conn = store.conn();
        conn.execute(
            "INSERT INTO docs (app, coll, id, owner, data, version, created_ms, updated_ms) \
             VALUES (?1, ?2, ?3, '1', '{}', 1, 0, 0)",
            params![app, coll, id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO docs_fts (app, coll, id, body) VALUES (?1, ?2, ?3, ?4)",
            params![app, coll, id, body],
        )
        .unwrap();
    }

    fn ids(store: &Store) -> Vec<String> {
        store
            .conn()
            .prepare("SELECT app || '/' || coll || '/' || id FROM docs ORDER BY 1")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    fn found(store: &Store, term: &str) -> Vec<String> {
        store
            .conn()
            .prepare("SELECT id FROM docs_fts WHERE docs_fts MATCH ?1 ORDER BY id")
            .unwrap()
            .query_map([term], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    /// **"I deleted the wrong recipes"**: one app's records come back from
    /// the backup, searchable again; another app's later writes stay, and
    /// what was replaced is kept.
    #[test]
    fn one_app_comes_back_and_the_rest_is_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("yenbot.db")).unwrap();
        let backups = dir.path().join("backups");
        put(&store, "recipes", "dishes", "r1", "pho");
        put(&store, "recipes", "dishes", "r2", "bun");
        put(&store, "recipes", "lists", "l1", "shopping");
        back_up(&store, &backups, 0).unwrap();

        store.conn().execute_batch("DELETE FROM docs WHERE app = 'recipes'; DELETE FROM docs_fts WHERE app = 'recipes'").unwrap();
        put(&store, "recipes", "lists", "l2", "market");
        put(&store, "home", "notes", "n1", "later");

        let day = file_for(&backups, 0);
        let dishes = ["dishes".to_string()];
        let r = app(&store, &day, "recipes", Some(&dishes), &backups, 7).unwrap();
        assert_eq!(r.records, 2);
        assert_eq!(r.replaced, backups.join("replaced-7.db"));
        assert!(r.replaced.is_file());
        let kept = ["home/notes/n1", "recipes/dishes/r1", "recipes/dishes/r2"];
        assert_eq!(ids(&store), [&kept[..], &["recipes/lists/l2"]].concat());
        assert_eq!(found(&store, "pho"), ["r1"]);

        let r = app(&store, &day, "recipes", None, &backups, 8).unwrap();
        assert_eq!(r.records, 3);
        assert_eq!(ids(&store), [&kept[..], &["recipes/lists/l1"]].concat());
        assert_eq!(found(&store, "market"), Vec::<String>::new());
    }

    /// **A corrupted store is replaced whole** and keeps serving.
    #[test]
    fn the_whole_store_comes_back() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("yenbot.db")).unwrap();
        let backups = dir.path().join("backups");
        put(&store, "home", "notes", "n1", "kept");
        back_up(&store, &backups, 0).unwrap();
        put(&store, "home", "notes", "n2", "lost");
        let r = whole(&store, &file_for(&backups, 0), &backups, 9).unwrap();
        assert_eq!(r.records, 1);
        assert_eq!(ids(&store), ["home/notes/n1"]);
        put(&store, "home", "notes", "n3", "after");
        assert_eq!(store.version().unwrap(), LATEST);
        let before = Store::open(&r.replaced).unwrap();
        assert_eq!(ids(&before), ["home/notes/n1", "home/notes/n2"]);
    }

    /// **A backup from a newer build is refused before anything is copied.**
    #[test]
    fn a_newer_backup_never_replaces_the_store() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("yenbot.db")).unwrap();
        put(&store, "home", "notes", "n1", "kept");
        let newer = dir.path().join("newer.db");
        let mut c = Connection::open(&newer).unwrap();
        migrate(&mut c).unwrap();
        let ahead = "INSERT INTO schema_version VALUES (?1)";
        c.execute(ahead, [LATEST + 1]).unwrap();
        drop(c);
        let e = whole(&store, &newer, dir.path(), 1).unwrap_err();
        assert!(e.to_string().contains("newer"), "{e}");
        assert_eq!(ids(&store), ["home/notes/n1"]);
    }

    /// **A backup from before records kept their shape restores them at the
    /// first**, for the app's migrations to take up.
    #[test]
    fn a_backup_from_before_shapes_restores_at_the_first() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("yenbot.db")).unwrap();
        let old = dir.path().join("old.db");
        let c = Connection::open(&old).unwrap();
        c.execute_batch("CREATE TABLE schema_version (version INTEGER NOT NULL);")
            .unwrap();
        for (i, step) in STEPS[..2].iter().enumerate() {
            c.execute_batch(step).unwrap();
            c.execute("INSERT INTO schema_version VALUES (?1)", [i + 1])
                .unwrap();
        }
        c.execute(
            "INSERT INTO docs (app, coll, id, owner, data, version, created_ms, updated_ms) \
             VALUES ('home', 'notes', 'n1', '1', '{}', 1, 0, 0)",
            [],
        )
        .unwrap();
        drop(c);
        assert_eq!(
            app(&store, &old, "home", None, dir.path(), 1)
                .unwrap()
                .records,
            1
        );
        let shape: u32 = store
            .conn()
            .query_row("SELECT shape FROM docs WHERE id = 'n1'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(shape, 1);
    }

    /// **A backup from before the records existed restores no app.**
    #[test]
    fn a_backup_without_records_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("yenbot.db")).unwrap();
        let old = dir.path().join("old.db");
        Connection::open(&old)
            .unwrap()
            .execute_batch("CREATE TABLE schema_version (version INTEGER NOT NULL); INSERT INTO schema_version VALUES (1);")
            .unwrap();
        let e = app(&store, &old, "home", None, dir.path(), 1).unwrap_err();
        assert!(e.to_string().contains("schema 1"), "{e}");
        assert!(ids(&store).is_empty(), "and the store is as it was");
    }
}
