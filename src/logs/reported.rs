//! What browsers reported (§18.1), as the admin page lists it: the newest
//! errors the framework posted, for one app or the whole site.

use anyhow::Result;
use rusqlite::{params, Connection};
use serde::Serialize;

use super::browser::source;

/// One error a browser reported.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Reported {
    pub at_ms: i64,
    /// `yenbot/<app>`, or `yenbot` for the platform's own pages.
    pub source: String,
    pub message: String,
}

/// The newest `limit` errors browsers reported, of `app` alone when one is
/// named (`_site` for the platform's pages).
pub fn reported(conn: &Connection, app: Option<&str>, limit: usize) -> Result<Vec<Reported>> {
    let source = app.map(source);
    let mut stmt = conn.prepare(
        "SELECT at_ms, source, message FROM log_lines WHERE target = 'browser' \
         AND (?1 IS NULL OR source = ?1) ORDER BY at_ms DESC, id DESC LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![source, limit as i64], |r| {
        Ok(Reported {
            at_ms: r.get(0)?,
            source: r.get(1)?,
            message: r.get(2)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;

    #[test]
    fn a_browsers_errors_are_listed_newest_first() {
        let store = Store::in_memory().unwrap();
        let conn = store.conn();
        conn.execute_batch(
            "INSERT INTO log_lines (at_ms, level, source, target, message) VALUES \
               (1, 'error', 'yenbot/recipes', 'browser', 'TypeError: a'), \
               (2, 'error', 'yenbot/chores', 'browser', 'TypeError: b'), \
               (3, 'info', 'yenbot/recipes', 'workflow', 'ran'), \
               (4, 'error', 'yenbot/recipes', 'browser', 'TypeError: c');",
        )
        .unwrap();
        let shown: Vec<String> = reported(&conn, Some("recipes"), 10)
            .unwrap()
            .into_iter()
            .map(|r| r.message)
            .collect();
        assert_eq!(shown, ["TypeError: c", "TypeError: a"]);
        assert_eq!(reported(&conn, None, 2).unwrap().len(), 2);
    }
}
