//! The inbox (§11.2): every notification written to its account's inbox
//! before it is pushed, so nothing is lost on a device that was off or never
//! subscribed. The bell shows what is unread.

use anyhow::Result;
use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};

use crate::data::docs::new_id;

/// The longest a title and a body may be, in characters.
pub const TITLE_MAX: usize = 120;
pub const BODY_MAX: usize = 1_000;

/// The most an inbox keeps of each account's, newest.
const KEPT: i64 = 500;

/// What a notification says.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Message {
    pub title: String,
    #[serde(default)]
    pub body: String,
    /// The address a tap opens — the app's own page, `/a/<app>/…`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open: Option<String>,
}

impl Message {
    /// The message with its title and body cut to their lengths, or why it
    /// is not one.
    pub fn checked(&self) -> std::result::Result<Message, &'static str> {
        let title: String = self.title.trim().chars().take(TITLE_MAX).collect();
        if title.is_empty() {
            return Err("a notification has a title");
        }
        if let Some(open) = &self.open {
            if !open.starts_with('/') || open.starts_with("//") {
                return Err("`open` is an address on this site, starting with `/`");
            }
        }
        Ok(Message {
            title,
            body: self.body.trim().chars().take(BODY_MAX).collect(),
            open: self.open.clone(),
        })
    }
}

/// One notification in an inbox.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Item {
    pub id: String,
    pub app: String,
    pub title: String,
    pub body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open: Option<String>,
    pub created_ms: i64,
    pub read_ms: Option<i64>,
}

fn item(r: &Row<'_>) -> rusqlite::Result<Item> {
    Ok(Item {
        id: r.get(0)?,
        app: r.get(1)?,
        title: r.get(2)?,
        body: r.get(3)?,
        open: r.get(4)?,
        created_ms: r.get(5)?,
        read_ms: r.get(6)?,
    })
}

/// Write `m`, from app `app`, to `account`'s inbox; answers its id. The
/// oldest beyond the newest five hundred go.
pub fn write(conn: &Connection, account: &str, app: &str, m: &Message, now: i64) -> Result<String> {
    let id = new_id(now);
    conn.execute(
        "INSERT INTO inbox (id, account, app, title, body, open, created_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![id, account, app, m.title, m.body, m.open, now],
    )?;
    conn.execute(
        "DELETE FROM inbox WHERE account = ?1 AND id NOT IN \
         (SELECT id FROM inbox WHERE account = ?1 ORDER BY created_ms DESC, id DESC LIMIT ?2)",
        params![account, KEPT],
    )?;
    Ok(id)
}

/// `account`'s newest `limit` notifications, newest first, and how many are
/// unread in all.
pub fn list(conn: &Connection, account: &str, limit: usize) -> Result<(Vec<Item>, u64)> {
    let mut stmt = conn.prepare(
        "SELECT id, app, title, body, open, created_ms, read_ms FROM inbox WHERE account = ?1 \
         ORDER BY created_ms DESC, id DESC LIMIT ?2",
    )?;
    let items = stmt
        .query_map(params![account, limit as i64], item)?
        .collect::<Result<_, _>>()?;
    let unread = conn.query_row(
        "SELECT COUNT(*) FROM inbox WHERE account = ?1 AND read_ms IS NULL",
        params![account],
        |r| r.get(0),
    )?;
    Ok((items, unread))
}

/// Mark notification `id` of `account`'s read — every one, for `all`;
/// answers how many it marked.
pub fn read(conn: &Connection, account: &str, id: &str, now: i64) -> Result<usize> {
    Ok(conn.execute(
        "UPDATE inbox SET read_ms = ?3 WHERE account = ?1 AND (id = ?2 OR ?2 = 'all') AND read_ms IS NULL",
        params![account, id, now],
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;

    fn account(conn: &Connection, id: &str) {
        conn.execute(
            "INSERT INTO accounts (id, provider, email, name, status, created_ms, seen_ms) \
             VALUES (?1, 'google', ?1, ?1, 'active', 0, 0)",
            [id],
        )
        .unwrap();
    }

    fn msg(title: &str) -> Message {
        Message {
            title: title.into(),
            body: "b".into(),
            open: Some("/a/chores/".into()),
        }
    }

    /// **A notification is in its account's inbox, unread until read**; no
    /// one else's.
    #[test]
    fn the_inbox_holds_each_accounts_own() {
        let store = Store::in_memory().unwrap();
        let conn = store.conn();
        account(&conn, "1");
        account(&conn, "2");
        let a = write(&conn, "1", "chores", &msg("Mai finished the dishes"), 10).unwrap();
        write(&conn, "1", "chores", &msg("Bo finished the bins"), 20).unwrap();
        write(&conn, "2", "chores", &msg("not Yen's"), 30).unwrap();
        let (items, unread) = list(&conn, "1", 10).unwrap();
        assert_eq!(
            items.iter().map(|i| i.title.as_str()).collect::<Vec<_>>(),
            ["Bo finished the bins", "Mai finished the dishes"]
        );
        assert_eq!(unread, 2);
        assert_eq!(read(&conn, "2", &a, 40).unwrap(), 0, "not theirs");
        assert_eq!(read(&conn, "1", &a, 40).unwrap(), 1);
        assert_eq!(list(&conn, "1", 10).unwrap().1, 1);
        assert_eq!(read(&conn, "1", "all", 50).unwrap(), 1);
        assert_eq!(list(&conn, "1", 10).unwrap().1, 0);
        assert_eq!(list(&conn, "1", 10).unwrap().0[1].read_ms, Some(40));
    }

    #[test]
    fn a_message_is_held_to_its_shape() {
        let long = "x".repeat(TITLE_MAX + 5);
        let m = Message {
            title: format!("  {long} "),
            body: String::new(),
            open: None,
        };
        assert_eq!(m.checked().unwrap().title.chars().count(), TITLE_MAX);
        let blank = Message {
            title: "  ".into(),
            ..m.clone()
        };
        assert_eq!(blank.checked(), Err("a notification has a title"));
        let away = Message {
            title: "t".into(),
            open: Some("https://elsewhere.example".into()),
            ..m.clone()
        };
        assert!(away.checked().is_err());
        let other = Message {
            title: "t".into(),
            open: Some("//elsewhere.example/x".into()),
            ..m
        };
        assert!(other.checked().is_err());
    }
}
