//! Notes to Yen from the apps' own menus (§6.1, §13.2): each is delivered
//! to the admins' inboxes and kept here, for the admin page to list by app.

use anyhow::Result;
use rusqlite::{params, Connection};
use serde::Serialize;

/// A note as the admin page lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Note {
    pub app: String,
    pub account: String,
    /// Who sent it, as their account is now named.
    pub name: Option<String>,
    pub text: String,
    pub at_ms: i64,
}

/// Keep `account`'s note `text` about `app`.
pub fn keep(conn: &Connection, app: &str, account: &str, text: &str, now: i64) -> Result<()> {
    conn.execute(
        "INSERT INTO feedback (app, account, text, at_ms) VALUES (?1, ?2, ?3, ?4)",
        params![app, account, text, now],
    )?;
    Ok(())
}

/// The newest `limit` notes, about `app` alone when one is named.
pub fn list(conn: &Connection, app: Option<&str>, limit: usize) -> Result<Vec<Note>> {
    let mut stmt = conn.prepare(
        "SELECT f.app, f.account, a.name, f.text, f.at_ms FROM feedback f \
         LEFT JOIN accounts a ON a.id = f.account \
         WHERE (?1 IS NULL OR f.app = ?1) ORDER BY f.at_ms DESC, f.id DESC LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![app, limit as i64], |r| {
        Ok(Note {
            app: r.get(0)?,
            account: r.get(1)?,
            name: r.get(2)?,
            text: r.get(3)?,
            at_ms: r.get(4)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;

    #[test]
    fn notes_are_kept_and_listed_newest_first() {
        let store = Store::in_memory().unwrap();
        let conn = store.conn();
        conn.execute(
            "INSERT INTO accounts (id, provider, email, name, status, created_ms, seen_ms) \
             VALUES ('2', 'google', 'mai@example.com', 'Mai', 'active', 0, 0)",
            [],
        )
        .unwrap();
        keep(&conn, "chores", "2", "Can it remind me too?", 5).unwrap();
        keep(&conn, "recipes", "2", "More soups", 6).unwrap();
        keep(&conn, "chores", "3", "Gone", 7).unwrap();
        let shown: Vec<_> = list(&conn, Some("chores"), 10)
            .unwrap()
            .into_iter()
            .map(|n| (n.name, n.text))
            .collect();
        assert_eq!(
            shown,
            [
                (None, "Gone".to_string()),
                (Some("Mai".to_string()), "Can it remind me too?".to_string())
            ]
        );
        assert_eq!(list(&conn, None, 1).unwrap()[0].text, "Gone");
    }
}
