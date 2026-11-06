//! Who may open an app (§12.4): every active account opens a live public
//! app; a private one opens for the accounts and groups it is shared with; a
//! draft opens for the admins alone, who open everything.

use anyhow::Result;
use rusqlite::{params, OptionalExtension};
use serde::Serialize;

use super::manifest::{Manifest, Status};
use crate::accounts::caller::Caller;
use crate::store::Store;

/// An app's visibility and its place on the landing page.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Access {
    pub private: bool,
    pub position: Option<i64>,
    /// `account:<id>` or `group:<name>`.
    pub shares: Vec<String>,
}

pub fn access_of(store: &Store, app: &str) -> Result<Access> {
    let conn = store.conn();
    let (private, position) = conn
        .query_row(
            "SELECT private, position FROM app_access WHERE app = ?1",
            params![app],
            |r| Ok((r.get::<_, bool>(0)?, r.get::<_, Option<i64>>(1)?)),
        )
        .optional()?
        .unwrap_or((false, None));
    let mut stmt = conn.prepare("SELECT who FROM app_shares WHERE app = ?1 ORDER BY who")?;
    let shares = stmt
        .query_map(params![app], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    Ok(Access {
        private,
        position,
        shares,
    })
}

/// Whether `caller` may open `app`.
pub fn may_open(store: &Store, app: &Manifest, caller: &Caller) -> Result<bool> {
    if caller.is_admin() {
        return Ok(true);
    }
    if app.status == Status::Draft {
        return Ok(false);
    }
    let a = access_of(store, &app.id)?;
    Ok(!a.private || caller.principals().iter().any(|p| a.shares.contains(p)))
}

/// Make `app` private or public.
pub fn set_private(store: &Store, app: &str, private: bool) -> Result<()> {
    store.conn().execute(
        "INSERT INTO app_access (app, private) VALUES (?1, ?2) \
         ON CONFLICT (app) DO UPDATE SET private = excluded.private",
        params![app, private],
    )?;
    Ok(())
}

/// Share a private `app` with exactly `who`.
pub fn set_shares(store: &Store, app: &str, who: &[String]) -> Result<()> {
    let mut conn = store.conn();
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM app_shares WHERE app = ?1", params![app])?;
    for w in who {
        tx.execute(
            "INSERT INTO app_shares (app, who) VALUES (?1, ?2)",
            params![app, w],
        )?;
    }
    tx.commit()?;
    Ok(())
}

/// Put the apps on the landing page in `order`.
pub fn set_order(store: &Store, order: &[String]) -> Result<()> {
    let mut conn = store.conn();
    let tx = conn.transaction()?;
    tx.execute("UPDATE app_access SET position = NULL", [])?;
    for (i, app) in order.iter().enumerate() {
        tx.execute(
            "INSERT INTO app_access (app, private, position) VALUES (?1, 0, ?2) \
             ON CONFLICT (app) DO UPDATE SET position = excluded.position",
            params![app, i as i64],
        )?;
    }
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounts::{Account, Status as AccountStatus};

    fn caller(id: &str, admin: bool, groups: &[&str]) -> Caller {
        Caller {
            account: Account {
                id: id.into(),
                email: String::new(),
                name: id.into(),
                picture: None,
                status: AccountStatus::Active,
                admin,
                created_ms: 0,
                seen_ms: 0,
            },
            groups: groups.iter().map(|g| g.to_string()).collect(),
            token: None,
        }
    }

    /// **Public to every account, private to whom it is shared with, a
    /// draft to the admins.**
    #[test]
    fn who_opens_an_app() {
        let store = Store::in_memory().unwrap();
        let live = Manifest::parse("id: chores\nname: Chores\nstatus: live\n", "chores").unwrap();
        let draft = Manifest::parse("id: wip\nname: Wip\n", "wip").unwrap();
        let (yen, kid, gran) = (
            caller("yen", true, &[]),
            caller("kid", false, &["household"]),
            caller("gran", false, &[]),
        );
        assert!(may_open(&store, &live, &kid).unwrap());
        assert!(!may_open(&store, &draft, &kid).unwrap());
        assert!(may_open(&store, &draft, &yen).unwrap());
        set_private(&store, "chores", true).unwrap();
        set_shares(&store, "chores", &["group:household".into()]).unwrap();
        assert!(may_open(&store, &live, &kid).unwrap());
        assert!(!may_open(&store, &live, &gran).unwrap());
        set_order(&store, &["chores".into(), "wip".into()]).unwrap();
        let a = access_of(&store, "chores").unwrap();
        assert_eq!(
            (a.private, a.position),
            (true, Some(0)),
            "ordering keeps it private"
        );
    }
}
