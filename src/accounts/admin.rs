//! What the admins do to accounts and groups (§12.2, §12.5): approve, refuse,
//! block and delete accounts, keep groups and who is in them, and set how a
//! new account becomes active.

use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use super::{Account, Status};
use crate::apps::manifest::is_name;
use crate::config::{Config, Registration};
use crate::store::Store;

/// What an admin does to an account.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Approve,
    /// A pending account turned away: it is removed, and may ask again.
    Refuse,
    Block,
    Delete,
}

impl Action {
    pub fn parse(s: &str) -> Option<Action> {
        Some(match s {
            "approve" => Action::Approve,
            "refuse" => Action::Refuse,
            "block" => Action::Block,
            "delete" => Action::Delete,
            _ => return None,
        })
    }
}

pub fn list(store: &Store, cfg: &Config) -> Result<Vec<Account>> {
    let conn = store.conn();
    let mut stmt = conn.prepare(
        "SELECT id, email, name, picture, status, created_ms, seen_ms FROM accounts ORDER BY created_ms",
    )?;
    let rows = stmt.query_map([], |r| {
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
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Do `action` to account `id`; whether there was one. Deleting an account
/// deletes its records in the `private` collections (app, collection); what
/// it wrote in shared ones stays, as a former member's (§20.1).
pub fn act(
    store: &Store,
    cfg: &Config,
    id: &str,
    action: Action,
    private: &[(String, String)],
) -> Result<bool> {
    let mut conn = store.conn();
    let email: Option<String> = conn
        .query_row(
            "SELECT email FROM accounts WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )
        .optional()?;
    let Some(email) = email else {
        return Ok(false);
    };
    if cfg.is_admin(&email) && action != Action::Approve {
        bail!("an admin's account is kept active; the admins are named in yenbot.yaml");
    }
    let changed = match action {
        Action::Approve => conn.execute(
            "UPDATE accounts SET status = 'active' WHERE id = ?1",
            params![id],
        )?,
        Action::Block => conn.execute(
            "UPDATE accounts SET status = 'blocked' WHERE id = ?1",
            params![id],
        )?,
        Action::Refuse => conn.execute(
            "DELETE FROM accounts WHERE id = ?1 AND status = 'pending'",
            params![id],
        )?,
        Action::Delete => {
            let tx = conn.transaction()?;
            for (app, coll) in private {
                tx.execute(
                    "DELETE FROM docs_fts WHERE app = ?1 AND coll = ?2 AND id IN \
                     (SELECT id FROM docs WHERE app = ?1 AND coll = ?2 AND owner = ?3)",
                    params![app, coll, id],
                )?;
                tx.execute(
                    "DELETE FROM docs WHERE app = ?1 AND coll = ?2 AND owner = ?3",
                    params![app, coll, id],
                )?;
            }
            unshare(&tx, &format!("account:{id}"))?;
            let n = tx.execute("DELETE FROM accounts WHERE id = ?1", params![id])?;
            tx.commit()?;
            n
        }
    };
    Ok(changed > 0)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Group {
    pub name: String,
    pub members: Vec<String>,
}

pub fn groups(store: &Store) -> Result<Vec<Group>> {
    let conn = store.conn();
    let mut stmt = conn.prepare(
        "SELECT g.name, m.account FROM groups g LEFT JOIN group_members m ON m.grp = g.name ORDER BY g.name, m.account",
    )?;
    let mut out: Vec<Group> = Vec::new();
    let rows = stmt.query_map([], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
    })?;
    for row in rows {
        let (name, member) = row?;
        if out.last().map(|g| &g.name) != Some(&name) {
            out.push(Group {
                name: name.clone(),
                members: Vec::new(),
            });
        }
        if let Some(m) = member {
            out.last_mut().expect("just pushed").members.push(m);
        }
    }
    Ok(out)
}

pub fn make_group(store: &Store, name: &str, now: i64) -> Result<()> {
    if !is_name(name) {
        bail!("`{name}` is not a group name: lowercase letters, digits, `-` and `_`");
    }
    store.conn().execute(
        "INSERT OR IGNORE INTO groups (name, created_ms) VALUES (?1, ?2)",
        params![name, now],
    )?;
    Ok(())
}

/// Delete group `name` and every share naming it, so a group made later
/// under the name inherits nothing.
pub fn drop_group(store: &Store, name: &str) -> Result<bool> {
    let mut conn = store.conn();
    let tx = conn.transaction()?;
    unshare(&tx, &format!("group:{name}"))?;
    let n = tx.execute("DELETE FROM groups WHERE name = ?1", params![name])?;
    tx.commit()?;
    Ok(n > 0)
}

/// Remove every record and app share naming `who`.
fn unshare(conn: &Connection, who: &str) -> Result<()> {
    conn.execute("DELETE FROM shares WHERE who = ?1", params![who])?;
    conn.execute("DELETE FROM app_shares WHERE who = ?1", params![who])?;
    Ok(())
}

pub fn add_member(store: &Store, group: &str, account: &str) -> Result<()> {
    store.conn().execute(
        "INSERT OR IGNORE INTO group_members (grp, account) VALUES (?1, ?2)",
        params![group, account],
    )?;
    Ok(())
}

pub fn remove_member(store: &Store, group: &str, account: &str) -> Result<bool> {
    Ok(store.conn().execute(
        "DELETE FROM group_members WHERE grp = ?1 AND account = ?2",
        params![group, account],
    )? > 0)
}

pub fn set_registration(store: &Store, r: Registration) -> Result<()> {
    let value = match r {
        Registration::Approve => "approve",
        Registration::Open => "open",
    };
    store.conn().execute(
        "INSERT INTO site_settings (key, value) VALUES ('registration', ?1) \
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        params![value],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::accounts::identity::Identity;
    use crate::accounts::{registration_of, sign_in};

    fn cfg() -> Config {
        Config::from_yaml(
            "data_dir: d\nlive_dir: l\nadmins: [yen@example.com]\ntime_zone: UTC\nbus: {url: u, key_file: k}\n",
            Path::new("."),
        )
        .unwrap()
    }

    fn person(sub: &str, email: &str) -> Identity {
        Identity {
            provider: "google".into(),
            sub: sub.into(),
            email: email.into(),
            name: "N".into(),
            picture: String::new(),
        }
    }

    #[test]
    fn an_admin_approves_refuses_and_blocks() {
        let (s, c) = (Store::in_memory().unwrap(), cfg());
        sign_in(&s, &c, &person("1", "yen@example.com"), 0).unwrap();
        sign_in(&s, &c, &person("2", "mai@example.com"), 0).unwrap();
        sign_in(&s, &c, &person("3", "spam@example.com"), 0).unwrap();
        assert!(act(&s, &c, "2", Action::Approve, &[]).unwrap());
        assert!(act(&s, &c, "3", Action::Refuse, &[]).unwrap());
        let held = list(&s, &c).unwrap();
        let states: Vec<_> = held.iter().map(|a| (a.id.as_str(), a.status)).collect();
        assert_eq!(states, [("1", Status::Active), ("2", Status::Active)]);
        assert!(act(&s, &c, "2", Action::Block, &[]).unwrap());
        let refused = act(&s, &c, "2", Action::Refuse, &[]).unwrap();
        assert!(!refused, "only a pending account is refused");
        let blocked = act(&s, &c, "1", Action::Block, &[]);
        assert!(blocked.is_err(), "an admin stays active");
        assert!(!act(&s, &c, "9", Action::Approve, &[]).unwrap());
    }

    /// **A deleted account loses its private records; its shared ones stay.**
    #[test]
    fn a_deleted_account_keeps_only_what_it_shared() {
        let (s, c) = (Store::in_memory().unwrap(), cfg());
        sign_in(&s, &c, &person("2", "mai@example.com"), 0).unwrap();
        for (coll, id) in [("notes", "n1"), ("recipes", "r1")] {
            s.conn()
                .execute(
                    "INSERT INTO docs (app, coll, id, owner, data, version, created_ms, updated_ms) \
                     VALUES ('home', ?1, ?2, '2', '{}', 1, 0, 0)",
                    params![coll, id],
                )
                .unwrap();
        }
        let private = [("home".to_string(), "notes".to_string())];
        assert!(act(&s, &c, "2", Action::Delete, &private).unwrap());
        let left: Vec<String> = s
            .conn()
            .prepare("SELECT id FROM docs")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(left, ["r1"]);
    }

    #[test]
    fn groups_hold_their_members() {
        let (s, c) = (Store::in_memory().unwrap(), cfg());
        sign_in(&s, &c, &person("2", "kid@example.com"), 0).unwrap();
        make_group(&s, "household", 0).unwrap();
        make_group(&s, "kids", 0).unwrap();
        add_member(&s, "kids", "2").unwrap();
        let group = |name: &str, members: &[&str]| Group {
            name: name.into(),
            members: members.iter().map(|m| m.to_string()).collect(),
        };
        assert_eq!(
            groups(&s).unwrap(),
            [group("household", &[]), group("kids", &["2"])]
        );
        assert!(remove_member(&s, "kids", "2").unwrap());
        s.conn()
            .execute_batch(
                "INSERT INTO shares VALUES ('home', 'notes', 'n1', 'group:kids', 0);
                 INSERT INTO app_shares VALUES ('games', 'group:kids');
                 INSERT INTO app_shares VALUES ('games', 'group:household');",
            )
            .unwrap();
        assert!(drop_group(&s, "kids").unwrap());
        let left: Vec<String> = s
            .conn()
            .prepare("SELECT who FROM shares UNION ALL SELECT who FROM app_shares")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(
            left,
            ["group:household"],
            "a group made again under the name inherits nothing"
        );
        assert!(make_group(&s, "Bad Name", 0).is_err());
        assert!(add_member(&s, "nobody", "2").is_err(), "no such group");
    }

    #[test]
    fn the_admin_page_sets_how_accounts_arrive() {
        let (s, c) = (Store::in_memory().unwrap(), cfg());
        let set = |s: &Store| registration_of(&s.conn(), &c).unwrap();
        assert_eq!(set(&s), Registration::Approve);
        set_registration(&s, Registration::Open).unwrap();
        assert_eq!(set(&s), Registration::Open);
        let (newcomer, _) = sign_in(&s, &c, &person("5", "new@example.com"), 0).unwrap();
        assert_eq!(newcomer.status, Status::Active);
    }
}
