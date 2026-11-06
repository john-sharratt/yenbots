//! Each write told to the open pages that may see it (§6.1, `GET
//! /api/v1/events/<app>`), so a list another person changes updates on every
//! screen showing it.
//!
//! A change names the record — its collection, id, what happened and the
//! version it is now at — never its fields: a page that hears of one reads it
//! through the API, which judges access as it judges every read. Who hears of
//! a change is decided when it is written ([`Audience`]): a shared
//! collection's readers, a private record's owner and those it is shared
//! with, or — for a deletion, which names only an id — everyone who may open
//! the app.

use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use tokio::sync::broadcast;

use super::docs::admits;
use crate::accounts::caller::Caller;
use crate::apps::manifest::{Manifest, Scope};

/// What happened to a record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Op {
    Created,
    Updated,
    Deleted,
}

/// Who may hear of a change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Audience {
    /// Whoever the collection's roles let read it.
    Readers,
    /// These principals (`account:<id>`, `group:<name>`).
    Principals(Vec<String>),
    /// Everyone who may open the app.
    App,
}

/// One record's change, as a page hears it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Changed {
    #[serde(skip)]
    pub app: String,
    pub coll: String,
    pub id: String,
    pub op: Op,
    /// The record's version after the write; 0 for a deletion.
    pub version: i64,
    #[serde(skip)]
    pub audience: Audience,
}

/// What the feed carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Feed {
    Changed(Changed),
    /// Anything may have changed — a restore — so the pages read it all
    /// again: one app's, or every app's when `None`.
    Reset(Option<String>),
}

impl Feed {
    /// Whether `caller`, with app `m` open, hears of it.
    pub fn reaches(&self, m: &Manifest, caller: &Caller) -> bool {
        match self {
            Feed::Reset(app) => app.as_deref().is_none_or(|a| a == m.id),
            Feed::Changed(c) if c.app != m.id => false,
            Feed::Changed(c) => match &c.audience {
                Audience::App => true,
                Audience::Readers => m
                    .collections
                    .get(&c.coll)
                    .is_some_and(|coll| admits(&coll.roles.read, caller)),
                Audience::Principals(p) => caller.principals().iter().any(|me| p.contains(me)),
            },
        }
    }
}

/// The feed every open page subscribes to.
#[derive(Clone)]
pub struct Changes {
    tx: broadcast::Sender<Feed>,
}

impl Changes {
    pub fn new() -> Changes {
        Changes {
            tx: broadcast::channel(256).0,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Feed> {
        self.tx.subscribe()
    }

    pub fn send(&self, feed: Feed) {
        // No page open is no one to tell.
        let _ = self.tx.send(feed);
    }
}

impl Default for Changes {
    fn default() -> Changes {
        Changes::new()
    }
}

/// Who hears of a write to record `id` of `coll`, and the version it is at,
/// read inside the write's transaction.
pub fn audience(conn: &Connection, m: &Manifest, coll: &str, id: &str) -> Result<(Audience, i64)> {
    let held: Option<(String, i64)> = conn
        .query_row(
            "SELECT owner, version FROM docs WHERE app = ?1 AND coll = ?2 AND id = ?3",
            params![m.id, coll, id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let (owner, version) = held.unwrap_or_default();
    let private = m
        .collections
        .get(coll)
        .is_some_and(|c| c.scope == Scope::Private);
    if !private {
        return Ok((Audience::Readers, version));
    }
    let mut who = vec![format!("account:{owner}")];
    let mut stmt = conn
        .prepare("SELECT who FROM shares WHERE app = ?1 AND coll = ?2 AND id = ?3 ORDER BY who")?;
    for w in stmt.query_map(params![m.id, coll, id], |r| r.get::<_, String>(0))? {
        who.push(w?);
    }
    Ok((Audience::Principals(who), version))
}

/// Both audiences: a record whose shares changed is told to those it was
/// shared with and those it now is, so the ones it left drop it.
pub fn either(a: Audience, b: Audience) -> Audience {
    match (a, b) {
        (Audience::Principals(mut a), Audience::Principals(b)) => {
            for w in b {
                if !a.contains(&w) {
                    a.push(w);
                }
            }
            Audience::Principals(a)
        }
        (Audience::App, _) | (_, Audience::App) => Audience::App,
        _ => Audience::Readers,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::accounts::{Account, Status};
    use crate::data::docs::{create, share, Shares};
    use crate::store::Store;

    const APP: &str = "id: home\nname: Home\ncollections:\n  notes:\n    scope: private\n    \
                       fields: { text: { type: text } }\n  chores:\n    roles: { read: [household] }\n    \
                       fields: { task: { type: text } }\n";

    fn caller(id: &str, groups: &[&str]) -> Caller {
        Caller {
            account: Account {
                id: id.into(),
                email: format!("{id}@example.com"),
                name: id.into(),
                picture: None,
                status: Status::Active,
                admin: false,
                created_ms: 0,
                seen_ms: 0,
            },
            groups: groups.iter().map(|g| g.to_string()).collect(),
            token: None,
        }
    }

    fn changed(coll: &str, audience: Audience) -> Feed {
        Feed::Changed(Changed {
            app: "home".into(),
            coll: coll.into(),
            id: "r1".into(),
            op: Op::Updated,
            version: 2,
            audience,
        })
    }

    /// **A change reaches only those who may read what changed.**
    #[test]
    fn a_change_reaches_its_audience_alone() {
        let m = Manifest::parse(APP, "home").unwrap();
        let (mum, kid) = (caller("mum", &["household"]), caller("kid", &[]));
        let chores = changed("chores", Audience::Readers);
        assert_eq!(
            (chores.reaches(&m, &mum), chores.reaches(&m, &kid)),
            (true, false)
        );
        let to_kid = changed("notes", Audience::Principals(vec!["account:kid".into()]));
        assert_eq!(
            (to_kid.reaches(&m, &mum), to_kid.reaches(&m, &kid)),
            (false, true)
        );
        let to_house = changed(
            "notes",
            Audience::Principals(vec!["group:household".into()]),
        );
        assert!(to_house.reaches(&m, &mum));
        assert!(changed("notes", Audience::App).reaches(&m, &kid));
        let other = Manifest::parse("id: other\nname: O\n", "other").unwrap();
        assert!(!changed("notes", Audience::App).reaches(&other, &kid));
        assert!(Feed::Reset(None).reaches(&other, &kid));
        assert!(!Feed::Reset(Some("home".into())).reaches(&other, &kid));
    }

    /// **A private record is heard by its owner and those it is shared
    /// with**; a shared collection's by its readers.
    #[test]
    fn a_records_audience_is_read_from_the_store() {
        let s = Store::in_memory().unwrap();
        let m = Manifest::parse(APP, "home").unwrap();
        let yen = caller("yen", &[]);
        let mut conn = s.conn();
        let tx = conn.transaction().unwrap();
        let note = create(&tx, &m, "notes", &yen, &json!({"text": "x"}), 1)
            .unwrap()
            .unwrap();
        let to = Shares {
            read: vec!["group:household".into()],
            edit: vec![],
        };
        share(&tx, &m, "notes", &yen, &note.id, &to)
            .unwrap()
            .unwrap();
        let (who, version) = audience(&tx, &m, "notes", &note.id).unwrap();
        let expected = vec!["account:yen".to_string(), "group:household".to_string()];
        assert_eq!((who, version), (Audience::Principals(expected), 1));
        assert_eq!(
            audience(&tx, &m, "chores", "any").unwrap(),
            (Audience::Readers, 0)
        );
    }

    #[test]
    fn either_audience_is_their_union() {
        let p = |w: &[&str]| Audience::Principals(w.iter().map(|s| s.to_string()).collect());
        assert_eq!(either(p(&["a", "b"]), p(&["b", "c"])), p(&["a", "b", "c"]));
        assert_eq!(
            either(Audience::Readers, Audience::Readers),
            Audience::Readers
        );
        assert_eq!(either(p(&["a"]), Audience::App), Audience::App);
    }
}
