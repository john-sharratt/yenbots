//! Files no record names (§20.1): a picture uploaded and never saved into
//! a record, a photo whose record was deleted. A platform job removes each
//! a week after it was kept, so a page or a workflow has time to save it.

use std::path::Path;

use anyhow::Result;
use rusqlite::{params, Connection};

use crate::files::Files;

/// How long a file is kept before it is judged.
pub const GRACE_MS: i64 = 7 * 86_400_000;

/// The files kept before `now - GRACE_MS` that no record of their app
/// names, as `(app, id)`; an archived app's are left for its purge.
pub fn unnamed(conn: &Connection, now: i64) -> Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT f.app, f.id FROM files f \
         WHERE f.created_ms < ?1 \
           AND f.app NOT IN (SELECT app FROM archived_apps) \
           AND NOT EXISTS (SELECT 1 FROM docs d WHERE d.app = f.app AND instr(d.data, f.id) > 0) \
         ORDER BY f.app, f.id",
    )?;
    let rows = stmt.query_map(params![now - GRACE_MS], |r| Ok((r.get(0)?, r.get(1)?)))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Remove each file `unnamed` names, its row and its bytes; answers how
/// many.
pub fn remove(conn: &Connection, files: &Files, now: i64) -> Result<usize> {
    let gone = unnamed(conn, now)?;
    for (app, id) in &gone {
        conn.execute(
            "DELETE FROM files WHERE app = ?1 AND id = ?2",
            params![app, id],
        )?;
        for thumb in [false, true] {
            remove_if_there(&files.path(app, id, thumb))?;
        }
    }
    Ok(gone.len())
}

fn remove_if_there(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;

    /// **A file is removed once a week has passed and no record names it**;
    /// a named one, a new one and an archived app's stay.
    #[test]
    fn a_file_no_record_names_is_removed() {
        let store = Store::in_memory().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let files = Files::new(dir.path());
        std::fs::create_dir_all(dir.path().join("recipes")).unwrap();
        for id in ["named", "unnamed", "young"] {
            std::fs::write(files.path("recipes", id, false), b"x").unwrap();
        }
        std::fs::write(files.path("recipes", "unnamed", true), b"t").unwrap();
        store
            .conn()
            .execute_batch(
                "INSERT INTO files (app, id, owner, name, kind, bytes, created_ms) VALUES \
                   ('recipes', 'named', '1', 'a', 'png', 1, 0), \
                   ('recipes', 'unnamed', '1', 'b', 'png', 1, 0), \
                   ('recipes', 'young', '1', 'c', 'png', 1, 604800000), \
                   ('old', 'gone', '1', 'd', 'png', 1, 0); \
                 INSERT INTO docs (app, coll, id, owner, data, version, created_ms, updated_ms) \
                   VALUES ('recipes', 'recipes', 'r1', '1', '{\"photo\":\"named\"}', 1, 0, 0); \
                 INSERT INTO archived_apps (app, reason, held_jobs, archived_ms, purge_ms) \
                   VALUES ('old', 'removed', '[]', 0, 0);",
            )
            .unwrap();
        let now = GRACE_MS + 1;
        let conn = store.conn();
        assert_eq!(
            unnamed(&conn, now).unwrap(),
            [("recipes".to_string(), "unnamed".to_string())]
        );
        assert_eq!(remove(&conn, &files, now).unwrap(), 1);
        assert!(!files.path("recipes", "unnamed", false).exists());
        assert!(!files.path("recipes", "unnamed", true).exists());
        assert!(files.path("recipes", "named", false).exists());
        assert!(unnamed(&conn, now).unwrap().is_empty());
    }
}
