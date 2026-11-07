//! A new account waiting to be let in (§12.2): Yen — the first admin
//! `yenbot.yaml` lists — is told, in her inbox and on her devices, with a tap
//! that opens the admin page's accounts, where either admin approves or
//! refuses it.

use anyhow::Result;
use rusqlite::{params, OptionalExtension};
use serde_json::json;

use super::devices::of;
use super::inbox::{write, Message};
use super::push::{Push, Pusher};
use crate::accounts::Account;
use crate::config::Config;
use crate::store::Store;

/// What the site's own notifications are from, in the inbox.
pub const SITE: &str = "_site";

/// Where the tap goes.
const OPEN: &str = "/_admin#accounts";

/// Tell the first admin that `account` is waiting; whether there was one to
/// tell — an admin who has not signed in yet has no inbox.
pub fn tell(
    store: &Store,
    cfg: &Config,
    pusher: &Pusher,
    account: &Account,
    now: i64,
) -> Result<bool> {
    let Some(first) = cfg.admins.first() else {
        return Ok(false);
    };
    let conn = store.conn();
    let to: Option<String> = conn
        .query_row(
            "SELECT id FROM accounts WHERE lower(email) = lower(?1) AND status = 'active'",
            params![first.trim()],
            |r| r.get(0),
        )
        .optional()?;
    let Some(to) = to else {
        return Ok(false);
    };
    let m = Message {
        title: format!("{} would like to join", account.name),
        body: format!(
            "{} is waiting: approve or refuse them on the admin page.",
            account.email
        ),
        open: Some(OPEN.to_string()),
    };
    let id = write(&conn, &to, SITE, &m, now)?;
    let payload = json!({
        "title": m.title,
        "body": m.body,
        "open": OPEN,
        "app": SITE,
        "id": id,
        "icon": "/_library/tiles/heart-coral-256.webp",
    })
    .to_string()
    .into_bytes();
    for device in of(&conn, &to)? {
        pusher.push(Push {
            to: device,
            payload: payload.clone(),
        });
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::accounts::Status;
    use crate::notify::devices::{subscribe, Keys, Subscription};
    use crate::notify::inbox::list;

    fn cfg() -> Config {
        Config::from_yaml(
            "data_dir: d\nlive_dir: l\nadmins: [yen@example.com, john@example.com]\n\
             time_zone: UTC\nbus: {url: u, key_file: k}\n",
            Path::new("."),
        )
        .unwrap()
    }

    fn account(id: &str, email: &str, status: &str) -> Account {
        Account {
            id: id.into(),
            email: email.into(),
            name: "Cousin Mai".into(),
            picture: None,
            status: if status == "active" {
                Status::Active
            } else {
                Status::Pending
            },
            admin: false,
            created_ms: 0,
            seen_ms: 0,
        }
    }

    fn insert(store: &Store, a: &Account) {
        let status = if a.status == Status::Active {
            "active"
        } else {
            "pending"
        };
        store
            .conn()
            .execute(
                "INSERT INTO accounts (id, provider, email, name, status, created_ms, seen_ms) \
                 VALUES (?1, 'google', ?2, ?3, ?4, 0, 0)",
                params![a.id, a.email, a.name, status],
            )
            .unwrap();
    }

    /// **Yen alone is told a newcomer waits — in her inbox and on her
    /// devices — and nobody is while she has never signed in.**
    #[test]
    fn the_first_admin_is_told_of_a_newcomer() {
        let store = Store::in_memory().unwrap();
        let (pusher, queued) = Pusher::channel();
        let cousin = account("9", "mai@example.com", "pending");
        insert(&store, &cousin);
        assert!(!tell(&store, &cfg(), &pusher, &cousin, 5).unwrap());

        insert(&store, &account("1", "Yen@Example.com", "active"));
        insert(&store, &account("2", "john@example.com", "active"));
        subscribe(
            &store.conn(),
            "1",
            &Subscription {
                endpoint: "https://fcm.googleapis.com/fcm/send/x".into(),
                keys: Keys {
                    p256dh: "p".into(),
                    auth: "a".into(),
                },
            },
            0,
        )
        .unwrap();
        assert!(tell(&store, &cfg(), &pusher, &cousin, 6).unwrap());
        let (items, unread) = list(&store.conn(), "1", 10).unwrap();
        assert_eq!(
            (items[0].title.as_str(), items[0].body.as_str(), unread),
            (
                "Cousin Mai would like to join",
                "mai@example.com is waiting: approve or refuse them on the admin page.",
                1
            )
        );
        assert_eq!(list(&store.conn(), "2", 10).unwrap().1, 0, "Yen alone");
        let pushed: Vec<Push> = queued.try_iter().collect();
        assert_eq!(pushed.len(), 1);
    }
}
