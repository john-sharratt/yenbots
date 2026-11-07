//! A small settings object per app and account (§6.1): what a person chose
//! in an app — a units toggle, the list they last had open — kept with them
//! across devices. `{}` until they choose something.

use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};

/// The most a settings object may hold, as JSON.
pub const MAX_BYTES: usize = 16 << 10;

/// `account`'s settings for `app`.
pub fn get(conn: &Connection, app: &str, account: &str) -> Result<Value> {
    let held: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE app = ?1 AND account = ?2",
            params![app, account],
            |r| r.get(0),
        )
        .optional()?;
    Ok(held
        .and_then(|v| serde_json::from_str(&v).ok())
        .unwrap_or_else(|| json!({})))
}

/// Replace `account`'s settings for `app` with `value`, an object.
pub fn put(conn: &Connection, app: &str, account: &str, value: &Value, now: i64) -> Result<()> {
    if !value.is_object() {
        bail!("settings are an object");
    }
    let text = value.to_string();
    if text.len() > MAX_BYTES {
        bail!("settings hold at most {} KB", MAX_BYTES >> 10);
    }
    conn.execute(
        "INSERT INTO settings (app, account, value, updated_ms) VALUES (?1, ?2, ?3, ?4) \
         ON CONFLICT (app, account) DO UPDATE SET value = ?3, updated_ms = ?4",
        params![app, account, text, now],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;

    #[test]
    fn settings_are_an_accounts_own_object() {
        let store = Store::in_memory().unwrap();
        let conn = store.conn();
        assert_eq!(get(&conn, "meals", "1").unwrap(), json!({}));
        put(&conn, "meals", "1", &json!({"units": "metric"}), 5).unwrap();
        put(&conn, "meals", "1", &json!({"units": "cups"}), 6).unwrap();
        assert_eq!(get(&conn, "meals", "1").unwrap(), json!({"units": "cups"}));
        assert_eq!(get(&conn, "meals", "2").unwrap(), json!({}));
        assert_eq!(
            put(&conn, "meals", "1", &json!([1]), 7)
                .unwrap_err()
                .to_string(),
            "settings are an object"
        );
        let big = json!({"x": "a".repeat(MAX_BYTES)});
        assert_eq!(
            put(&conn, "meals", "1", &big, 8).unwrap_err().to_string(),
            "settings hold at most 16 KB"
        );
    }
}
