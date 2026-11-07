//! A notification delivered (§11): to an account, a group, or the admins —
//! only those who may open the app it is from (§11.3) — written to each
//! one's inbox, and pushed to each of their devices unless they muted the
//! app.

use std::collections::BTreeSet;

use anyhow::Result;
use rusqlite::params;
use serde::Serialize;
use serde_json::json;

use super::devices::{muted, of};
use super::inbox::{write, Message};
use super::push::{Push, Pusher};
use crate::accounts::caller::{acting_for, Caller};
use crate::apps::access::may_open;
use crate::apps::manifest::Manifest;
use crate::config::Config;
use crate::store::Store;

/// Everyone who is an admin.
pub const ADMINS: &str = "admins";

/// What a notification is from, and where it may go.
pub struct Origin<'a> {
    pub store: &'a Store,
    pub cfg: &'a Config,
    pub pusher: &'a Pusher,
    /// The app it is from.
    pub app: &'a Manifest,
}

/// What a delivery did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Delivered {
    /// The inboxes it was written to.
    pub accounts: usize,
    /// The devices it was pushed to.
    pub devices: usize,
}

/// The accounts `to` names: an account's id, `group:<name>`, or `admins`.
fn named(from: &Origin<'_>, to: &str) -> Result<Vec<String>> {
    let conn = from.store.conn();
    if to == ADMINS {
        let mut stmt =
            conn.prepare("SELECT id, email FROM accounts WHERE status = 'active' ORDER BY id")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        let mut out = Vec::new();
        for row in rows {
            let (id, email) = row?;
            if from.cfg.is_admin(&email) {
                out.push(id);
            }
        }
        return Ok(out);
    }
    if let Some(group) = to.strip_prefix("group:") {
        let mut stmt =
            conn.prepare("SELECT account FROM group_members WHERE grp = ?1 ORDER BY account")?;
        let rows = stmt.query_map(params![group], |r| r.get(0))?;
        return Ok(rows.collect::<Result<_, _>>()?);
    }
    Ok(vec![to.to_string()])
}

/// The JSON the service worker shows.
fn payload(m: &Message, app: &Manifest, inbox_id: &str) -> Vec<u8> {
    let icon = app
        .tile
        .as_deref()
        .and_then(|t| t.strip_prefix("library/"))
        .map(|key| format!("/_library/{key}-256.webp"));
    json!({
        "title": m.title,
        "body": m.body,
        "open": m.open.clone().unwrap_or_else(|| format!("/a/{}/", app.id)),
        "app": app.id,
        "id": inbox_id,
        "icon": icon,
    })
    .to_string()
    .into_bytes()
}

/// Deliver `m` to `to`; why not, when `to` names no one it may reach.
pub fn deliver(
    from: &Origin<'_>,
    to: &str,
    m: &Message,
    now: i64,
) -> Result<std::result::Result<Delivered, String>> {
    let m = match m.checked() {
        Ok(m) => m,
        Err(why) => return Ok(Err(why.to_string())),
    };
    let mut reached: BTreeSet<String> = BTreeSet::new();
    let mut devices = 0;
    for id in named(from, to)? {
        let Some(caller): Option<Caller> = acting_for(from.store, from.cfg, &id)? else {
            continue;
        };
        if !may_open(from.store, from.app, &caller)? || !reached.insert(id.clone()) {
            continue;
        }
        let conn = from.store.conn();
        let inbox_id = write(&conn, &id, &from.app.id, &m, now)?;
        if muted(&conn, &id)?.contains(&from.app.id) {
            continue;
        }
        for device in of(&conn, &id)? {
            from.pusher.push(Push {
                to: device,
                payload: payload(&m, from.app, &inbox_id),
            });
            devices += 1;
        }
    }
    if reached.is_empty() {
        return Ok(Err(format!(
            "`{to}` names no active account that may open {}",
            from.app.id
        )));
    }
    Ok(Ok(Delivered {
        accounts: reached.len(),
        devices,
    }))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::notify::devices::{mute, subscribe, Keys, Subscription};
    use crate::notify::inbox::list;

    fn cfg() -> Config {
        Config::from_yaml(
            "data_dir: d\nlive_dir: l\nadmins: [yen@example.com]\ntime_zone: UTC\nbus: {url: u, key_file: k}\n",
            Path::new("."),
        )
        .unwrap()
    }

    fn setup() -> Store {
        let store = Store::in_memory().unwrap();
        let conn = store.conn();
        for (id, email, status) in [
            ("1", "yen@example.com", "active"),
            ("2", "mai@example.com", "active"),
            ("3", "bo@example.com", "pending"),
        ] {
            conn.execute(
                "INSERT INTO accounts (id, provider, email, name, status, created_ms, seen_ms) \
                 VALUES (?1, 'google', ?2, ?1, ?3, 0, 0)",
                params![id, email, status],
            )
            .unwrap();
        }
        conn.execute_batch(
            "INSERT INTO groups (name, created_ms) VALUES ('kids', 0); \
             INSERT INTO group_members (grp, account) VALUES ('kids', '2'), ('kids', '3');",
        )
        .unwrap();
        let device = |e: &str| Subscription {
            endpoint: e.into(),
            keys: Keys {
                p256dh: "k".into(),
                auth: "a".into(),
            },
        };
        subscribe(&conn, "1", &device("https://push/yen-phone"), 0).unwrap();
        subscribe(&conn, "1", &device("https://push/yen-laptop"), 0).unwrap();
        subscribe(&conn, "2", &device("https://push/mai-phone"), 0).unwrap();
        drop(conn);
        store
    }

    fn chores() -> Manifest {
        Manifest::parse(
            "id: chores\nname: Chores\nstatus: live\ntile: library/tiles/checklist-teal\n",
            "chores",
        )
        .unwrap()
    }

    fn msg() -> Message {
        Message {
            title: "Mai finished the dishes".into(),
            body: "Ticked off at 6:10 pm".into(),
            open: None,
        }
    }

    /// **The admins are told on every device; the inbox holds it either
    /// way; a muted app writes to the inbox and pushes nothing.**
    #[test]
    fn a_notification_reaches_the_inbox_and_each_device() {
        let store = setup();
        let (pusher, queued) = Pusher::channel();
        let app = chores();
        let cfg = cfg();
        let from = Origin {
            store: &store,
            cfg: &cfg,
            pusher: &pusher,
            app: &app,
        };
        assert_eq!(
            deliver(&from, ADMINS, &msg(), 10).unwrap(),
            Ok(Delivered {
                accounts: 1,
                devices: 2
            })
        );
        let pushed: Vec<Push> = queued.try_iter().collect();
        assert_eq!(pushed.len(), 2);
        let (items, unread) = list(&store.conn(), "1", 10).unwrap();
        assert_eq!(
            (items[0].title.as_str(), unread),
            ("Mai finished the dishes", 1)
        );
        let shown: serde_json::Value = serde_json::from_slice(&pushed[0].payload).unwrap();
        assert_eq!(
            shown,
            json!({"title": "Mai finished the dishes", "body": "Ticked off at 6:10 pm", "open": "/a/chores/",
                   "app": "chores", "id": items[0].id, "icon": "/_library/tiles/checklist-teal-256.webp"})
        );

        mute(&store.conn(), "1", "chores", true).unwrap();
        assert_eq!(
            deliver(&from, "1", &msg(), 20).unwrap(),
            Ok(Delivered {
                accounts: 1,
                devices: 0
            })
        );
        assert_eq!(queued.try_iter().count(), 0);
        assert_eq!(
            list(&store.conn(), "1", 10).unwrap().1,
            2,
            "still in the inbox"
        );
    }

    /// **A group reaches its active members; no one it may not reach is
    /// told, and naming no one is said.**
    #[test]
    fn only_those_who_may_open_the_app_are_told() {
        let store = setup();
        let (pusher, queued) = Pusher::channel();
        let cfg = cfg();
        let app = chores();
        let from = Origin {
            store: &store,
            cfg: &cfg,
            pusher: &pusher,
            app: &app,
        };
        assert_eq!(
            deliver(&from, "group:kids", &msg(), 10).unwrap(),
            Ok(Delivered {
                accounts: 1,
                devices: 1
            }),
            "Bo is not approved yet"
        );
        assert_eq!(
            queued.try_iter().next().unwrap().to.endpoint,
            "https://push/mai-phone"
        );
        assert_eq!(
            deliver(&from, "3", &msg(), 10).unwrap(),
            Err("`3` names no active account that may open chores".to_string())
        );
        let draft = Manifest::parse("id: chores\nname: Chores\n", "chores").unwrap();
        let from = Origin {
            app: &draft,
            ..from
        };
        assert_eq!(
            deliver(&from, "2", &msg(), 10).unwrap(),
            Err("`2` names no active account that may open chores".to_string()),
            "a draft opens for the admins alone"
        );
        let blank = Message {
            title: " ".into(),
            ..msg()
        };
        assert_eq!(
            deliver(&from, "1", &blank, 10).unwrap(),
            Err("a notification has a title".to_string())
        );
    }
}
