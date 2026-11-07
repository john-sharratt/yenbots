//! The devices an account has allowed notifications on (§11.1): one Web
//! Push subscription each — the push service's endpoint and the device's
//! key and auth secret — and the apps the account has muted (§11.3).

use anyhow::Result;
use rusqlite::{params, Connection};
use serde::Deserialize;

/// A subscription as a browser's `PushSubscription.toJSON()` gives it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Subscription {
    pub endpoint: String,
    pub keys: Keys,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Keys {
    pub p256dh: String,
    pub auth: String,
}

/// Keep `s` for `account`; a device subscribing again replaces its own.
pub fn subscribe(conn: &Connection, account: &str, s: &Subscription, now: i64) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO push_subscriptions (endpoint, account, p256dh, auth, created_ms) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![s.endpoint, account, s.keys.p256dh, s.keys.auth, now],
    )?;
    Ok(())
}

/// Forget the device at `endpoint`, when it is `account`'s.
pub fn unsubscribe(conn: &Connection, account: &str, endpoint: &str) -> Result<usize> {
    Ok(conn.execute(
        "DELETE FROM push_subscriptions WHERE endpoint = ?1 AND account = ?2",
        params![endpoint, account],
    )?)
}

/// Forget the device at `endpoint`, whoever's: its push service says it is
/// gone.
pub fn gone(conn: &Connection, endpoint: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM push_subscriptions WHERE endpoint = ?1",
        params![endpoint],
    )?;
    Ok(())
}

/// The push service took a message for `endpoint`.
pub fn sent(conn: &Connection, endpoint: &str, now: i64) -> Result<()> {
    conn.execute(
        "UPDATE push_subscriptions SET sent_ms = ?2 WHERE endpoint = ?1",
        params![endpoint, now],
    )?;
    Ok(())
}

/// `account`'s devices.
pub fn of(conn: &Connection, account: &str) -> Result<Vec<Subscription>> {
    let mut stmt = conn.prepare(
        "SELECT endpoint, p256dh, auth FROM push_subscriptions WHERE account = ?1 ORDER BY created_ms",
    )?;
    let rows = stmt.query_map(params![account], |r| {
        Ok(Subscription {
            endpoint: r.get(0)?,
            keys: Keys {
                p256dh: r.get(1)?,
                auth: r.get(2)?,
            },
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Mute app `app`'s notifications for `account`, or not: a muted app still
/// writes to the inbox, and pushes nothing.
pub fn mute(conn: &Connection, account: &str, app: &str, muted: bool) -> Result<()> {
    if muted {
        conn.execute(
            "INSERT OR IGNORE INTO mutes (account, app) VALUES (?1, ?2)",
            params![account, app],
        )?;
    } else {
        conn.execute(
            "DELETE FROM mutes WHERE account = ?1 AND app = ?2",
            params![account, app],
        )?;
    }
    Ok(())
}

/// The apps `account` has muted.
pub fn muted(conn: &Connection, account: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT app FROM mutes WHERE account = ?1 ORDER BY app")?;
    let rows = stmt.query_map(params![account], |r| r.get(0))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;

    fn sub(endpoint: &str) -> Subscription {
        Subscription {
            endpoint: endpoint.into(),
            keys: Keys {
                p256dh: "k".into(),
                auth: "a".into(),
            },
        }
    }

    #[test]
    fn devices_and_mutes_are_kept_per_account() {
        let store = Store::in_memory().unwrap();
        let conn = store.conn();
        for id in ["1", "2"] {
            conn.execute(
                "INSERT INTO accounts (id, provider, email, name, status, created_ms, seen_ms) \
                 VALUES (?1, 'google', ?1, ?1, 'active', 0, 0)",
                [id],
            )
            .unwrap();
        }
        subscribe(&conn, "1", &sub("https://push/a"), 1).unwrap();
        subscribe(&conn, "1", &sub("https://push/b"), 2).unwrap();
        subscribe(&conn, "1", &sub("https://push/a"), 3).unwrap();
        assert_eq!(
            of(&conn, "1").unwrap(),
            [sub("https://push/b"), sub("https://push/a")]
        );
        assert_eq!(
            unsubscribe(&conn, "2", "https://push/a").unwrap(),
            0,
            "not theirs"
        );
        assert_eq!(unsubscribe(&conn, "1", "https://push/a").unwrap(), 1);
        gone(&conn, "https://push/b").unwrap();
        assert!(of(&conn, "1").unwrap().is_empty());

        mute(&conn, "1", "chores", true).unwrap();
        mute(&conn, "1", "chores", true).unwrap();
        mute(&conn, "1", "meals", true).unwrap();
        assert_eq!(muted(&conn, "1").unwrap(), ["chores", "meals"]);
        mute(&conn, "1", "chores", false).unwrap();
        assert_eq!(muted(&conn, "1").unwrap(), ["meals"]);
        assert!(muted(&conn, "2").unwrap().is_empty());
    }
}
