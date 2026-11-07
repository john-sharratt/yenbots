//! Accounts (`candle/docs/yenbot_app_host.md` §12.2): a person's first
//! signed-in request registers one, keyed by their Google subject, and every
//! later one refreshes their name and picture. A new account is `pending`
//! until an admin approves it — unless registration is open, or the person
//! is an admin, whose account is active from the first request.

pub mod admin;
pub mod caller;
pub mod identity;
pub mod tokens;

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use crate::config::{Config, Registration};
use crate::store::Store;
use identity::Identity;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Pending,
    Active,
    Blocked,
}

impl Status {
    fn as_str(self) -> &'static str {
        match self {
            Status::Pending => "pending",
            Status::Active => "active",
            Status::Blocked => "blocked",
        }
    }

    fn parse(s: &str) -> Status {
        match s {
            "active" => Status::Active,
            "blocked" => Status::Blocked,
            _ => Status::Pending,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Account {
    pub id: String,
    pub email: String,
    pub name: String,
    pub picture: Option<String>,
    pub status: Status,
    pub admin: bool,
    pub created_ms: i64,
    pub seen_ms: i64,
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

/// How a new account becomes active: as the admin page last set it, else
/// as `yenbot.yaml` says.
pub fn registration_of(conn: &Connection, cfg: &Config) -> Result<Registration> {
    let set: Option<String> = conn
        .query_row(
            "SELECT value FROM site_settings WHERE key = 'registration'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    Ok(match set.as_deref() {
        Some("open") => Registration::Open,
        Some("approve") => Registration::Approve,
        _ => cfg.registration,
    })
}

/// The account `id` is signed in as: registered on its first request,
/// refreshed on every later one.
pub fn sign_in(store: &Store, cfg: &Config, id: &Identity, now: i64) -> Result<Account> {
    let admin = cfg.is_admin(&id.email);
    let conn = store.conn();
    let held: Option<String> = conn
        .query_row(
            "SELECT status FROM accounts WHERE id = ?1",
            params![id.sub],
            |r| r.get(0),
        )
        .optional()?;
    let picture = (!id.picture.is_empty()).then(|| id.picture.clone());
    match held {
        Some(status) => {
            // An admin's account is always active: the list in the config
            // is the one place admin is decided.
            let status = if admin {
                Status::Active
            } else {
                Status::parse(&status)
            };
            conn.execute(
                "UPDATE accounts SET email = ?2, name = ?3, picture = ?4, seen_ms = ?5, status = ?6 \
                 WHERE id = ?1",
                params![id.sub, id.email, id.name, picture, now, status.as_str()],
            )?;
        }
        None => {
            let registration = registration_of(&conn, cfg)?;
            let status = if admin || registration == Registration::Open {
                Status::Active
            } else {
                Status::Pending
            };
            conn.execute(
                "INSERT INTO accounts (id, provider, email, name, picture, status, created_ms, seen_ms) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
                params![id.sub, id.provider, id.email, id.name, picture, status.as_str(), now],
            )?;
            tracing::info!(email = %id.email, status = status.as_str(), "accounts: registered");
        }
    }
    drop(conn);
    get(store, cfg, &id.sub)?.ok_or_else(|| anyhow!("the account just written is not there"))
}

/// The account `id`, if there is one.
pub fn get(store: &Store, cfg: &Config, id: &str) -> Result<Option<Account>> {
    let conn = store.conn();
    Ok(conn
        .query_row(
            "SELECT id, email, name, picture, status, created_ms, seen_ms FROM accounts WHERE id = ?1",
            params![id],
            |r| {
                let email: String = r.get(1)?;
                Ok(Account {
                    id: r.get(0)?,
                    admin: cfg.is_admin(&email),
                    email,
                    name: r.get(2)?,
                    picture: r.get(3)?,
                    status: Status::parse(&r.get::<_, String>(4)?),
                    created_ms: r.get(5)?,
                    seen_ms: r.get(6)?,
                })
            },
        )
        .optional()?)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn cfg(registration: &str) -> Config {
        Config::from_yaml(
            &format!(
                "data_dir: d\nlive_dir: l\nadmins: [yen@example.com]\nregistration: {registration}\n\
                 time_zone: UTC\nbus: {{url: u, key_file: k}}\n"
            ),
            Path::new("."),
        )
        .unwrap()
    }

    fn person(sub: &str, email: &str) -> Identity {
        Identity {
            provider: "google".into(),
            sub: sub.into(),
            email: email.into(),
            name: "Name".into(),
            picture: String::new(),
        }
    }

    /// **A newcomer waits for approval; an admin does not**, and a return
    /// visit keeps the account, refreshing its name.
    #[test]
    fn a_newcomer_waits_and_an_admin_does_not() {
        let store = Store::in_memory().unwrap();
        let c = cfg("approve");
        let cousin = sign_in(&store, &c, &person("2", "cousin@example.com"), 10).unwrap();
        assert_eq!((cousin.status, cousin.admin), (Status::Pending, false));
        let yen = sign_in(&store, &c, &person("1", "Yen@Example.com"), 11).unwrap();
        assert_eq!((yen.status, yen.admin), (Status::Active, true));

        let mut renamed = person("2", "cousin@example.com");
        renamed.name = "Cousin Mai".into();
        let again = sign_in(&store, &c, &renamed, 20).unwrap();
        assert_eq!(
            (
                again.status,
                again.name.as_str(),
                again.created_ms,
                again.seen_ms
            ),
            (Status::Pending, "Cousin Mai", 10, 20)
        );
    }

    #[test]
    fn open_registration_admits_everyone_at_once() {
        let store = Store::in_memory().unwrap();
        let a = sign_in(&store, &cfg("open"), &person("3", "a@example.com"), 1).unwrap();
        assert_eq!(a.status, Status::Active);
    }
}
