//! Who a request acts as (§6, §6.3, §12): the account the gateway's identity
//! names, or the one an API token belongs to — never a blocked or pending
//! account for anything but `/me` — with the groups it is in.

use anyhow::Result;
use axum::http::{header, HeaderMap};
use rusqlite::params;

use super::identity::identify;
use super::{get, now_ms, sign_in, tokens, Account, Status};
use crate::config::Config;
use crate::store::Store;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Caller {
    pub account: Account,
    pub groups: Vec<String>,
    /// The API token it came with, by name.
    pub token: Option<String>,
}

/// The start of the id an app acts under when a workflow runs as the app
/// itself: `app:reminders`. No account's id has it — an account's is its
/// Google subject, digits.
pub const APP_PREFIX: &str = "app:";

impl Caller {
    /// App `id` itself, as a workflow running for no account acts (§10.2):
    /// it reads and writes every record of its own collections, and a record
    /// it makes is owned by `app:<id>`.
    pub fn app(id: &str, name: &str) -> Caller {
        Caller {
            account: Account {
                id: format!("{APP_PREFIX}{id}"),
                email: String::new(),
                name: name.to_string(),
                picture: None,
                status: Status::Active,
                admin: false,
                created_ms: 0,
                seen_ms: 0,
            },
            groups: Vec::new(),
            token: None,
        }
    }

    /// Whether this is an app acting as itself.
    pub fn is_app(&self) -> bool {
        self.account.id.starts_with(APP_PREFIX)
    }

    pub fn id(&self) -> &str {
        &self.account.id
    }

    pub fn is_admin(&self) -> bool {
        self.account.admin
    }

    /// The names a share or a role may give it: `account:<id>`, and
    /// `group:<name>` for each group.
    pub fn principals(&self) -> Vec<String> {
        let mut out = vec![format!("account:{}", self.account.id)];
        out.extend(self.groups.iter().map(|g| format!("group:{g}")));
        out
    }
}

/// The groups account `id` is in.
pub fn groups_of(store: &Store, id: &str) -> Result<Vec<String>> {
    let conn = store.conn();
    let mut stmt = conn.prepare("SELECT grp FROM group_members WHERE account = ?1 ORDER BY grp")?;
    let rows = stmt.query_map(params![id], |r| r.get(0))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Account `id` as a job acting for it calls (§9.1): `None` once it is no
/// longer active — blocked, or gone.
pub fn acting_for(store: &Store, cfg: &Config, id: &str) -> Result<Option<Caller>> {
    let Some(account) = get(store, cfg, id)? else {
        return Ok(None);
    };
    if account.status != Status::Active {
        return Ok(None);
    }
    let groups = groups_of(store, &account.id)?;
    Ok(Some(Caller {
        account,
        groups,
        token: None,
    }))
}

/// What a request's caller is, before access is judged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Who {
    /// Nobody is signed in, or a token is not one.
    Nobody,
    /// Signed in, and not yet approved or blocked: `/me` alone answers it.
    Waiting(Account),
    Active(Caller),
}

/// The caller of a request with `headers`, registering a signed-in person's
/// account on first sight.
pub fn who(store: &Store, cfg: &Config, headers: &HeaderMap) -> Result<Who> {
    let bearer = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    let (account, token) = match (bearer, identify(headers)) {
        (Some(secret), _) => match tokens::holder(store, secret, now_ms())? {
            Some((id, name)) => match get(store, cfg, &id)? {
                Some(account) => (account, Some(name)),
                None => return Ok(Who::Nobody),
            },
            None => return Ok(Who::Nobody),
        },
        (None, Some(id)) => (sign_in(store, cfg, &id, now_ms())?, None),
        (None, None) => return Ok(Who::Nobody),
    };
    if account.status != Status::Active {
        return Ok(Who::Waiting(account));
    }
    let groups = groups_of(store, &account.id)?;
    Ok(Who::Active(Caller {
        account,
        groups,
        token,
    }))
}
