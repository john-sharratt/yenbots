//! An app's integration keys (§19.1): set and replaced on the admin page,
//! listed there by name, never shown again; a workflow reads one with
//! `ctx.secret(name)`, and no route answers one to a browser.

use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use crate::apps::manifest::is_name;

/// The longest a key may be.
pub const MAX_VALUE: usize = 8 << 10;

/// One key, as the admin page lists it: its name and when it was set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Held {
    pub name: String,
    pub set_ms: i64,
}

/// Set `app`'s key `name` to `value`, replacing what it was.
pub fn set(conn: &Connection, app: &str, name: &str, value: &str, now: i64) -> Result<()> {
    if !is_name(name) {
        bail!("`{name}` is not a key's name: lowercase letters, digits, `-` and `_`");
    }
    if value.is_empty() || value.len() > MAX_VALUE {
        bail!("a key is 1 to {MAX_VALUE} bytes");
    }
    conn.execute(
        "INSERT INTO secrets (app, name, value, set_ms) VALUES (?1, ?2, ?3, ?4) \
         ON CONFLICT (app, name) DO UPDATE SET value = ?3, set_ms = ?4",
        params![app, name, value, now],
    )?;
    Ok(())
}

/// Remove `app`'s key `name`; whether there was one.
pub fn remove(conn: &Connection, app: &str, name: &str) -> Result<bool> {
    Ok(conn.execute(
        "DELETE FROM secrets WHERE app = ?1 AND name = ?2",
        params![app, name],
    )? > 0)
}

/// `app`'s keys by name — never their values.
pub fn names(conn: &Connection, app: &str) -> Result<Vec<Held>> {
    let mut stmt = conn.prepare("SELECT name, set_ms FROM secrets WHERE app = ?1 ORDER BY name")?;
    let rows = stmt.query_map(params![app], |r| {
        Ok(Held {
            name: r.get(0)?,
            set_ms: r.get(1)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// `app`'s key `name`, for a workflow or a hook's signature.
pub fn get(conn: &Connection, app: &str, name: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT value FROM secrets WHERE app = ?1 AND name = ?2",
            params![app, name],
            |r| r.get(0),
        )
        .optional()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;

    /// **A key is set, replaced, listed by name alone, read for its app
    /// only, and removed.**
    #[test]
    fn a_key_is_set_replaced_listed_and_removed() {
        let store = Store::in_memory().unwrap();
        let conn = store.conn();
        set(&conn, "weather", "api-key", "k1", 5).unwrap();
        set(&conn, "weather", "api-key", "k2", 9).unwrap();
        set(&conn, "weather", "city", "Sydney", 7).unwrap();
        assert_eq!(
            names(&conn, "weather").unwrap(),
            [
                Held {
                    name: "api-key".into(),
                    set_ms: 9
                },
                Held {
                    name: "city".into(),
                    set_ms: 7
                }
            ]
        );
        assert_eq!(
            get(&conn, "weather", "api-key").unwrap().as_deref(),
            Some("k2")
        );
        assert_eq!(get(&conn, "other", "api-key").unwrap(), None);
        assert!(remove(&conn, "weather", "api-key").unwrap());
        assert!(!remove(&conn, "weather", "api-key").unwrap());
        assert_eq!(get(&conn, "weather", "api-key").unwrap(), None);
    }

    #[test]
    fn a_key_has_a_name_and_a_value() {
        let store = Store::in_memory().unwrap();
        let conn = store.conn();
        let told = |name: &str, value: &str| {
            set(&conn, "a", name, value, 0)
                .map_err(|e| e.to_string())
                .err()
        };
        assert_eq!(
            told("API key", "x").as_deref(),
            Some("`API key` is not a key's name: lowercase letters, digits, `-` and `_`")
        );
        assert_eq!(told("k", "").as_deref(), Some("a key is 1 to 8192 bytes"));
        assert_eq!(
            told("k", &"x".repeat(MAX_VALUE + 1)).as_deref(),
            Some("a key is 1 to 8192 bytes")
        );
        assert_eq!(told("k", &"x".repeat(MAX_VALUE)), None);
    }
}
