//! `/api/v1/admin/…` (§6.2): what the admin page shows and does — accounts,
//! groups, apps, every account's records, API tokens, registration and
//! backups — the admins' alone.

use std::sync::Arc;

use anyhow::Error;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::Response;
use serde::Deserialize;
use serde_json::{json, Value};

use super::reply::{admin, body, denied, error, internal, ok, Args};
use crate::accounts::admin::{self as accounts, Action};
use crate::accounts::caller::Caller;
use crate::accounts::{now_ms, tokens};
use crate::app::App;
use crate::apps::access::{access_of, set_order, set_private, set_shares};
use crate::apps::manifest::Scope;
use crate::config::Registration;
use crate::data::changes::Feed;
use crate::data::docs::{self, Query as DocQuery};
use crate::files::mirror::bring_back;
use crate::jobs::scheduler::sync_all;
use crate::jobs::sync::sync;
use crate::jobs::table::requeue_running;
use crate::reshape;
use crate::store::{backups, restore};

/// Run `work` on the blocking pool for an admin caller.
pub(super) async fn as_admin<F>(
    s: Arc<App>,
    method: Method,
    headers: HeaderMap,
    work: F,
) -> Response
where
    F: FnOnce(&App, &Caller) -> Response + Send + 'static,
{
    match admin(&s, &method, &headers).await {
        Ok(c) => tokio::task::spawn_blocking(move || work(&s, &c))
            .await
            .unwrap_or_else(internal),
        Err(r) => r,
    }
}

pub(super) fn field<T: for<'de> Deserialize<'de>>(raw: &[u8], key: &str) -> Result<T, Response> {
    let v = body(raw)?;
    serde_json::from_value(v.get(key).cloned().unwrap_or(Value::Null)).map_err(|e| {
        error(
            StatusCode::BAD_REQUEST,
            "bad_request",
            format!("`{key}`: {e}"),
        )
    })
}

/// The whole body as `T`, or a 400.
fn whole<T: for<'de> Deserialize<'de>>(raw: &[u8]) -> Result<T, Response> {
    serde_json::from_value(body(raw)?)
        .map_err(|e| error(StatusCode::BAD_REQUEST, "bad_request", e.to_string()))
}

pub(super) fn no_app(id: &str) -> Response {
    error(
        StatusCode::NOT_FOUND,
        "no_app",
        format!("there is no app `{id}`"),
    )
}

pub async fn accounts_list(
    State(s): State<Arc<App>>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    as_admin(s, method, headers, |s, _| {
        match accounts::list(&s.store, &s.cfg) {
            Ok(a) => ok(json!({"accounts": a})),
            Err(e) => internal(e),
        }
    })
    .await
}

/// The private collections of every app, as `(app, collection)`.
fn private_collections(s: &App) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for m in s.apps.all() {
        for (name, c) in &m.collections {
            if c.scope == Scope::Private {
                out.push((m.id.clone(), name.clone()));
            }
        }
    }
    out
}

pub async fn accounts_act(
    State(s): State<Arc<App>>,
    Path((id, action)): Path<(String, String)>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    as_admin(s, method, headers, move |s, _| {
        let Some(action) = Action::parse(&action) else {
            return error(
                StatusCode::NOT_FOUND,
                "no_action",
                "approve, refuse, block or delete",
            );
        };
        let private = private_collections(s);
        match accounts::act(&s.store, &s.cfg, &id, action, &private) {
            Ok(true) => ok(json!({"account": id, "done": true})),
            Ok(false) => error(
                StatusCode::NOT_FOUND,
                "not_found",
                "no such account, or not one this applies to",
            ),
            Err(e) => error(StatusCode::CONFLICT, "refused", e.to_string()),
        }
    })
    .await
}

pub async fn groups_list(
    State(s): State<Arc<App>>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    as_admin(s, method, headers, |s, _| {
        match accounts::groups(&s.store) {
            Ok(g) => ok(json!({"groups": g})),
            Err(e) => internal(e),
        }
    })
    .await
}

pub async fn groups_make(
    State(s): State<Arc<App>>,
    method: Method,
    headers: HeaderMap,
    raw: Bytes,
) -> Response {
    let name: String = match field(&raw, "name") {
        Ok(n) => n,
        Err(r) => return r,
    };
    as_admin(s, method, headers, move |s, _| {
        match accounts::make_group(&s.store, &name, now_ms()) {
            Ok(()) => ok(json!({"group": name})),
            Err(e) => error(StatusCode::BAD_REQUEST, "bad_name", e.to_string()),
        }
    })
    .await
}

pub async fn groups_drop(
    State(s): State<Arc<App>>,
    Path(name): Path<String>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    as_admin(s, method, headers, move |s, _| {
        match accounts::drop_group(&s.store, &name) {
            Ok(true) => ok(json!({"deleted": name})),
            Ok(false) => error(StatusCode::NOT_FOUND, "not_found", "no such group"),
            Err(e) => internal(e),
        }
    })
    .await
}

pub async fn members_add(
    State(s): State<Arc<App>>,
    Path(name): Path<String>,
    method: Method,
    headers: HeaderMap,
    raw: Bytes,
) -> Response {
    let account: String = match field(&raw, "account") {
        Ok(a) => a,
        Err(r) => return r,
    };
    as_admin(s, method, headers, move |s, _| {
        match accounts::add_member(&s.store, &name, &account) {
            Ok(()) => ok(json!({"group": name, "added": account})),
            Err(e) => error(StatusCode::BAD_REQUEST, "refused", e.to_string()),
        }
    })
    .await
}

pub async fn members_remove(
    State(s): State<Arc<App>>,
    Path((name, account)): Path<(String, String)>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    as_admin(
        s,
        method,
        headers,
        move |s, _| match accounts::remove_member(&s.store, &name, &account) {
            Ok(true) => ok(json!({"group": name, "removed": account})),
            Ok(false) => error(StatusCode::NOT_FOUND, "not_found", "not a member"),
            Err(e) => internal(e),
        },
    )
    .await
}

pub async fn apps_list(State(s): State<Arc<App>>, method: Method, headers: HeaderMap) -> Response {
    as_admin(s, method, headers, |s, _| {
        let mut out = Vec::new();
        for m in s.apps.all() {
            let access = match access_of(&s.store, &m.id) {
                Ok(a) => a,
                Err(e) => return internal(e),
            };
            let collections: Vec<&String> = m.collections.keys().collect();
            out.push(json!({
                "id": m.id,
                "name": m.name,
                "status": m.status,
                "access": access,
                "collections": collections,
            }));
        }
        ok(json!({"apps": out}))
    })
    .await
}

/// `GET /admin/apps/{id}/check`: the app's last check on load (§17.4) — ok,
/// or each refusal with its rule, file and line, and the commit served
/// meanwhile. An app refused the first time it was seen, and so not served,
/// is answered too.
pub async fn apps_check(
    State(s): State<Arc<App>>,
    Path(id): Path<String>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    as_admin(s, method, headers, move |s, _| match s.apps.checked(&id) {
        None => no_app(&id),
        Some(c) => match reshape::of_app(&s.store.conn(), &id) {
            Ok(migrations) => ok(json!({
                "app": id,
                "ok": c.refusals.is_empty(),
                "commit": c.commit,
                "serving": c.serving,
                "refusals": c.refusals,
                "migrating": c.migrating,
                "migrations": migrations,
            })),
            Err(e) => internal(e),
        },
    })
    .await
}

pub async fn apps_visibility(
    State(s): State<Arc<App>>,
    Path(id): Path<String>,
    method: Method,
    headers: HeaderMap,
    raw: Bytes,
) -> Response {
    let private: bool = match field(&raw, "private") {
        Ok(p) => p,
        Err(r) => return r,
    };
    as_admin(s, method, headers, move |s, _| {
        if s.apps.get(&id).is_none() {
            return no_app(&id);
        }
        match set_private(&s.store, &id, private) {
            Ok(()) => ok(json!({"app": id, "private": private})),
            Err(e) => internal(e),
        }
    })
    .await
}

pub async fn apps_shares(
    State(s): State<Arc<App>>,
    Path(id): Path<String>,
    method: Method,
    headers: HeaderMap,
    raw: Bytes,
) -> Response {
    let who: Vec<String> = match field(&raw, "who") {
        Ok(w) => w,
        Err(r) => return r,
    };
    let principal = |w: &&String| w.starts_with("account:") || w.starts_with("group:");
    if let Some(bad) = who.iter().find(|w| !principal(w)) {
        return error(
            StatusCode::BAD_REQUEST,
            "bad_share",
            format!("`{bad}` is not `account:<id>` or `group:<name>`"),
        );
    }
    as_admin(s, method, headers, move |s, _| {
        match set_shares(&s.store, &id, &who) {
            Ok(()) => ok(json!({"app": id, "shares": who})),
            Err(e) => internal(e),
        }
    })
    .await
}

pub async fn apps_order(
    State(s): State<Arc<App>>,
    method: Method,
    headers: HeaderMap,
    raw: Bytes,
) -> Response {
    let order: Vec<String> = match field(&raw, "order") {
        Ok(o) => o,
        Err(r) => return r,
    };
    as_admin(s, method, headers, move |s, _| {
        match set_order(&s.store, &order) {
            Ok(()) => ok(json!({"order": order})),
            Err(e) => internal(e),
        }
    })
    .await
}

pub async fn data_all(
    State(s): State<Arc<App>>,
    Path((app, coll)): Path<(String, String)>,
    Args(q): Args<DocQuery>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    as_admin(s, method, headers, move |s, c| {
        let Some(m) = s.apps.get(&app) else {
            return no_app(&app);
        };
        match docs::query(&s.store.conn(), &m, &coll, c, &q, true) {
            Ok(Ok(page)) => ok(page),
            Ok(Err(d)) => denied(d),
            Err(e) => internal(e),
        }
    })
    .await
}

pub async fn tokens_list(
    State(s): State<Arc<App>>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    as_admin(s, method, headers, |s, _| match tokens::list(&s.store) {
        Ok(t) => ok(json!({"tokens": t})),
        Err(e) => internal(e),
    })
    .await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NewToken {
    name: String,
    #[serde(default = "a_year")]
    days: i64,
}

fn a_year() -> i64 {
    365
}

pub async fn tokens_make(
    State(s): State<Arc<App>>,
    method: Method,
    headers: HeaderMap,
    raw: Bytes,
) -> Response {
    let t: NewToken = match whole(&raw) {
        Ok(t) => t,
        Err(r) => return r,
    };
    as_admin(s, method, headers, move |s, c| {
        match tokens::create(&s.store, &t.name, c.id(), t.days, now_ms()) {
            Ok(secret) => ok(json!({
                "name": t.name,
                "secret": secret,
                "note": "shown once: save it as a Zen credential now",
            })),
            Err(e) => error(StatusCode::BAD_REQUEST, "refused", e.to_string()),
        }
    })
    .await
}

pub async fn tokens_revoke(
    State(s): State<Arc<App>>,
    Path(name): Path<String>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    as_admin(s, method, headers, move |s, _| {
        match tokens::revoke(&s.store, &name) {
            Ok(true) => ok(json!({"revoked": name})),
            Ok(false) => error(StatusCode::NOT_FOUND, "not_found", "no such token"),
            Err(e) => internal(e),
        }
    })
    .await
}

pub async fn backups_list(
    State(s): State<Arc<App>>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    as_admin(s, method, headers, |s, _| {
        match backups::list(&s.cfg.backups_dir()) {
            Ok(b) => ok(json!({"backups": b})),
            Err(e) => internal(e),
        }
    })
    .await
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct Restore {
    #[serde(default)]
    app: Option<String>,
    #[serde(default)]
    collections: Option<Vec<String>>,
}

/// After a whole store came back: its running jobs due again, then every
/// app's jobs brought into step — the store taken for one, then the other.
fn requeued(s: &App, now: i64) -> Result<(), Error> {
    let conn = s.store.conn();
    requeue_running(&conn, now)?;
    drop(conn);
    sync_all(s, now)
}

pub async fn backups_restore(
    State(s): State<Arc<App>>,
    Path(date): Path<String>,
    method: Method,
    headers: HeaderMap,
    raw: Bytes,
) -> Response {
    let asked: Restore = if raw.is_empty() {
        Restore::default()
    } else {
        match whole(&raw) {
            Ok(r) => r,
            Err(r) => return r,
        }
    };
    as_admin(s, method, headers, move |s, _| {
        let dir = s.cfg.backups_dir();
        let Some(from) = backups::held_file(&dir, &date) else {
            return error(
                StatusCode::NOT_FOUND,
                "no_backup",
                format!("there is no backup of `{date}`"),
            );
        };
        let done = match (&asked.app, &asked.collections) {
            (Some(app), colls) => {
                if s.apps.get(app).is_none() {
                    return no_app(app);
                }
                restore::app(&s.store, &from, app, colls.as_deref(), &dir, now_ms())
            }
            (None, Some(_)) => {
                return error(
                    StatusCode::BAD_REQUEST,
                    "bad_request",
                    "`collections` are an app's: name the `app`",
                )
            }
            (None, None) => restore::whole(&s.store, &from, &dir, now_ms()),
        };
        match done {
            Ok(r) => {
                // What came back has its jobs again — a reminder's with its
                // record — and every open page of it reads it all again.
                let now = now_ms();
                if let Some(app) = &asked.app {
                    if let Err(e) = bring_back(&s.cfg.kept_files_dir(), &s.cfg.files_dir(), app, &r.files) {
                        tracing::error!(target: "yenbot::store", app = %app, "its files could not all be brought back: {e:#}");
                    }
                }
                // A whole store comes back with the jobs that were running
                // when it was written — the backup's own among them — which
                // are due again, as after a restart.
                let resynced = match asked.app.as_deref().and_then(|a| s.apps.get(a)) {
                    Some(m) => sync(&mut s.store.conn(), &m, &s.cfg.zone(), now),
                    None => requeued(s, now),
                };
                if let Err(e) = resynced {
                    tracing::error!(target: "yenbot::jobs", "after the restore, its jobs could not be brought into step: {e:#}");
                }
                s.jobs.wake();
                s.changes.send(Feed::Reset(asked.app.clone()));
                ok(json!({
                "restored": date,
                "app": asked.app,
                "records": r.records,
                "replaced": r.replaced.file_name().map(|n| n.to_string_lossy().to_string()),
                }))
            }
            Err(e) => error(StatusCode::CONFLICT, "refused", format!("{e:#}")),
        }
    })
    .await
}

pub async fn registration(
    State(s): State<Arc<App>>,
    method: Method,
    headers: HeaderMap,
    raw: Bytes,
) -> Response {
    let r: Registration = match field(&raw, "registration") {
        Ok(r) => r,
        Err(r) => return r,
    };
    as_admin(
        s,
        method,
        headers,
        move |s, _| match accounts::set_registration(&s.store, r) {
            Ok(()) => ok(json!({"registration": r})),
            Err(e) => internal(e),
        },
    )
    .await
}
