//! An app renamed (§20.1): its folder moved to the new id with
//! `renamed_from: <old>` in its manifest, in one commit. On load the site
//! moves everything of the old id's to the new — records, files, jobs,
//! access, settings, keys — once, and serves the old address as a redirect.

use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};

use super::tables::APP_TABLES;

/// Whether `old` was renamed already.
pub fn done(conn: &Connection, old: &str) -> Result<bool> {
    Ok(conn
        .query_row("SELECT 1 FROM renames WHERE old = ?1", params![old], |_| {
            Ok(())
        })
        .optional()?
        .is_some())
}

/// The id app `old` is now, when it was renamed.
pub fn now_named(conn: &Connection, old: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT new FROM renames WHERE old = ?1",
            params![old],
            |r| r.get(0),
        )
        .optional()?)
}

/// Move everything of `old`'s to `new`, and its files under `files_dir`;
/// ids renamed to `old` before now redirect to `new`.
pub fn take(conn: &mut Connection, files_dir: &Path, old: &str, new: &str, now: i64) -> Result<()> {
    let tx = conn.transaction()?;
    for table in APP_TABLES {
        tx.execute(
            &format!("UPDATE {table} SET app = ?2 WHERE app = ?1"),
            params![old, new],
        )?;
    }
    tx.execute("DELETE FROM archived_apps WHERE app = ?1", params![old])?;
    tx.execute(
        "UPDATE renames SET new = ?2 WHERE new = ?1",
        params![old, new],
    )?;
    tx.execute(
        "INSERT INTO renames (old, new, at_ms) VALUES (?1, ?2, ?3)",
        params![old, new, now],
    )?;
    tx.commit()?;
    let (from, to) = (files_dir.join(old), files_dir.join(new));
    if from.exists() && !to.exists() {
        std::fs::rename(&from, &to)
            .with_context(|| format!("moving {} to {}", from.display(), to.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;

    /// **A rename moves the old id's records, jobs, access and files to the
    /// new, once, and an older name follows it.**
    #[test]
    fn a_rename_moves_everything_once() {
        let store = Store::in_memory().unwrap();
        store
            .conn()
            .execute_batch(
                "INSERT INTO docs (app, coll, id, owner, data, version, created_ms, updated_ms) \
                   VALUES ('recipe', 'recipes', 'r1', '1', '{}', 1, 0, 0); \
                 INSERT INTO jobs (id, app, workflow, args, source, state, updated_ms) \
                   VALUES ('j1', 'recipe', 'remind', '{}', 'schedule:0', 'waiting', 0); \
                 INSERT INTO app_access (app, private) VALUES ('recipe', 1); \
                 INSERT INTO renames (old, new, at_ms) VALUES ('food', 'recipe', 0);",
            )
            .unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("recipe")).unwrap();
        std::fs::write(dir.path().join("recipe").join("f1"), b"x").unwrap();

        assert!(!done(&store.conn(), "recipe").unwrap());
        take(&mut store.conn(), dir.path(), "recipe", "recipes", 5).unwrap();
        let conn = store.conn();
        for (table, n) in [("docs", 1), ("jobs", 1), ("app_access", 1)] {
            let moved: i64 = conn
                .query_row(
                    &format!("SELECT COUNT(*) FROM {table} WHERE app = 'recipes'"),
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(moved, n, "{table}");
        }
        assert!(done(&conn, "recipe").unwrap());
        assert_eq!(
            now_named(&conn, "recipe").unwrap().as_deref(),
            Some("recipes")
        );
        assert_eq!(
            now_named(&conn, "food").unwrap().as_deref(),
            Some("recipes")
        );
        assert_eq!(now_named(&conn, "recipes").unwrap(), None);
        assert!(dir.path().join("recipes").join("f1").exists());
        assert!(!dir.path().join("recipe").exists());
    }
}
