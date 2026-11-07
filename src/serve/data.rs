//! `/api/v1/data/…` and `/api/v1/apps…` (§6.1): an app's records, read and
//! written as the caller — each write told to the open pages that may see it
//! once it is committed — and the apps the caller may open.

use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use rusqlite::{Connection, Transaction};
use serde::Deserialize;
use serde_json::{json, Map, Value};

use super::reply::{active, body, denied, error, internal, ok, Args};
use crate::accounts::caller::Caller;
use crate::accounts::now_ms;
use crate::app::App;
use crate::apps::access::{access_of, may_open};
use crate::apps::manifest::{Manifest, Scope};
use crate::data::changes::{change_of, either, Changed, Feed, Op};
use crate::data::docs::{self, admits, Change, Query as DocQuery, Shares, Sum, Write, Wrote};

pub fn no_app(id: &str) -> Response {
    error(
        StatusCode::NOT_FOUND,
        "no_app",
        format!("there is no app `{id}`"),
    )
}

/// Where app `m`'s tile is served at the landing page's size: its library
/// picture (`library/tiles/<id>`, §15) as 256 wide WebP.
pub fn tile_url(m: &Manifest) -> Option<String> {
    m.tile
        .as_deref()
        .and_then(|t| t.strip_prefix("library/"))
        .map(|key| format!("/_library/{key}-256.webp"))
}

/// The app `id`, when the caller may open it. One the caller may not open
/// is answered as one that is not there.
pub fn opened(state: &App, id: &str, caller: &Caller) -> Result<Arc<Manifest>, Response> {
    let m = state.apps.get(id).ok_or_else(|| no_app(id))?;
    match may_open(&state.store, &m, caller) {
        Ok(true) => Ok(m),
        Ok(false) => Err(no_app(id)),
        Err(e) => Err(internal(e)),
    }
}

/// Run `work` on the blocking pool with the caller and the app.
pub(super) async fn with<F>(
    state: Arc<App>,
    method: Method,
    headers: HeaderMap,
    app: String,
    work: F,
) -> Response
where
    F: FnOnce(&App, &Caller, &Manifest) -> Response + Send + 'static,
{
    let caller = match active(&state, &method, &headers).await {
        Ok(c) => c,
        Err(r) => return r,
    };
    tokio::task::spawn_blocking(move || match opened(&state, &app, &caller) {
        Ok(m) => work(&state, &caller, &m),
        Err(r) => r,
    })
    .await
    .unwrap_or_else(internal)
}

/// The change `op` made to record `id` of `coll`, with who hears of it,
/// read inside the write's transaction.
fn changed(
    conn: &Connection,
    m: &Manifest,
    coll: &str,
    id: &str,
    op: Op,
) -> Result<Changed, Response> {
    change_of(conn, m, coll, id, op).map_err(internal)
}

/// Commit `tx`, then tell the open pages of each of `changes`.
fn commit(s: &App, tx: Transaction<'_>, changes: Vec<Changed>) -> Result<(), Response> {
    tx.commit().map_err(internal)?;
    for c in changes {
        s.changes.send(Feed::Changed(c));
    }
    Ok(())
}

pub async fn list(
    State(s): State<Arc<App>>,
    Path((app, coll)): Path<(String, String)>,
    Args(q): Args<DocQuery>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    with(s, method, headers, app, move |s, c, m| {
        match docs::query(&s.store.conn(), m, &coll, c, &q, false) {
            Ok(Ok(page)) => ok(page),
            Ok(Err(d)) => denied(d),
            Err(e) => internal(e),
        }
    })
    .await
}

pub async fn create(
    State(s): State<Arc<App>>,
    Path((app, coll)): Path<(String, String)>,
    method: Method,
    headers: HeaderMap,
    raw: Bytes,
) -> Response {
    let data = match body(&raw) {
        Ok(v) => v,
        Err(r) => return r,
    };
    with(s, method, headers, app, move |s, c, m| {
        let mut conn = s.store.conn();
        let tx = match conn.transaction() {
            Ok(tx) => tx,
            Err(e) => return internal(e),
        };
        let doc = match docs::create(&tx, m, &coll, c, &data, now_ms()) {
            Ok(Ok(doc)) => doc,
            Ok(Err(d)) => return denied(d),
            Err(e) => return internal(e),
        };
        let told = changed(&tx, m, &coll, &doc.id, Op::Created);
        match told.and_then(|t| commit(s, tx, vec![t])) {
            Ok(()) => (StatusCode::CREATED, Json(doc)).into_response(),
            Err(r) => r,
        }
    })
    .await
}

pub async fn read(
    State(s): State<Arc<App>>,
    Path((app, coll, id)): Path<(String, String, String)>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    with(s, method, headers, app, move |s, c, m| {
        match docs::get(&s.store.conn(), m, &coll, c, &id) {
            Ok(Ok(doc)) => ok(doc),
            Ok(Err(d)) => denied(d),
            Err(e) => internal(e),
        }
    })
    .await
}

/// The version an `If-Match` names: `"3"` or `3`.
fn if_match(headers: &HeaderMap) -> Result<Option<i64>, Response> {
    let Some(v) = headers.get(header::IF_MATCH).and_then(|v| v.to_str().ok()) else {
        return Ok(None);
    };
    v.trim().trim_matches('"').parse().map(Some).map_err(|_| {
        error(
            StatusCode::BAD_REQUEST,
            "bad_if_match",
            "If-Match names the record's version, a number",
        )
    })
}

pub async fn patch(
    State(s): State<Arc<App>>,
    Path((app, coll, id)): Path<(String, String, String)>,
    method: Method,
    headers: HeaderMap,
    raw: Bytes,
) -> Response {
    let (data, expect) = match (body(&raw), if_match(&headers)) {
        (Ok(d), Ok(e)) => (d, e),
        (Err(r), _) | (_, Err(r)) => return r,
    };
    with(s, method, headers, app, move |s, c, m| {
        let mut conn = s.store.conn();
        let tx = match conn.transaction() {
            Ok(tx) => tx,
            Err(e) => return internal(e),
        };
        let change = Change {
            patch: &data,
            expect,
        };
        let doc = match docs::update(&tx, m, &coll, c, &id, change, now_ms()) {
            Ok(Ok(doc)) => doc,
            Ok(Err(d)) => return denied(d),
            Err(e) => return internal(e),
        };
        let told = changed(&tx, m, &coll, &id, Op::Updated);
        match told.and_then(|t| commit(s, tx, vec![t])) {
            Ok(()) => ok(doc),
            Err(r) => r,
        }
    })
    .await
}

pub async fn remove(
    State(s): State<Arc<App>>,
    Path((app, coll, id)): Path<(String, String, String)>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    with(s, method, headers, app, move |s, c, m| {
        let mut conn = s.store.conn();
        let tx = match conn.transaction() {
            Ok(tx) => tx,
            Err(e) => return internal(e),
        };
        match docs::delete(&tx, m, &coll, c, &id, now_ms()) {
            Ok(Ok(())) => {}
            Ok(Err(d)) => return denied(d),
            Err(e) => return internal(e),
        }
        let told = changed(&tx, m, &coll, &id, Op::Deleted);
        match told.and_then(|t| commit(s, tx, vec![t])) {
            Ok(()) => ok(json!({"deleted": id})),
            Err(r) => r,
        }
    })
    .await
}

pub async fn sum(
    State(s): State<Arc<App>>,
    Path((app, coll)): Path<(String, String)>,
    Args(q): Args<Sum>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    with(s, method, headers, app, move |s, c, m| {
        match docs::sum(&s.store.conn(), m, &coll, c, &q) {
            Ok(Ok(rows)) => ok(json!({"groups": rows})),
            Ok(Err(d)) => denied(d),
            Err(e) => internal(e),
        }
    })
    .await
}

#[derive(Deserialize)]
pub struct SearchQuery {
    q: String,
    #[serde(default)]
    limit: Option<usize>,
}

pub async fn search(
    State(s): State<Arc<App>>,
    Path((app, coll)): Path<(String, String)>,
    Args(q): Args<SearchQuery>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    with(s, method, headers, app, move |s, c, m| {
        match docs::search(&s.store.conn(), m, &coll, c, &q.q, q.limit.unwrap_or(20)) {
            Ok(Ok(found)) => ok(json!({"docs": found})),
            Ok(Err(d)) => denied(d),
            Err(e) => internal(e),
        }
    })
    .await
}

/// The changes a batch made, one per write.
fn batch_changes(
    conn: &Connection,
    m: &Manifest,
    writes: &[Write],
    done: &[Wrote],
) -> Result<Vec<Changed>, Response> {
    let mut out = Vec::new();
    for (w, d) in writes.iter().zip(done) {
        let c = match (w, d) {
            (Write::Create { coll, .. }, Wrote::Doc(doc)) => {
                changed(conn, m, coll, &doc.id, Op::Created)?
            }
            (Write::Update { coll, id, .. }, _) => changed(conn, m, coll, id, Op::Updated)?,
            (Write::Delete { coll, id }, _) => changed(conn, m, coll, id, Op::Deleted)?,
            (Write::Create { .. }, Wrote::Deleted { .. }) => {
                return Err(internal("a create answered as a deletion"))
            }
        };
        out.push(c);
    }
    Ok(out)
}

pub async fn batch(
    State(s): State<Arc<App>>,
    Path(app): Path<String>,
    method: Method,
    headers: HeaderMap,
    raw: Bytes,
) -> Response {
    let writes: Vec<Write> = match body(&raw).and_then(|v| {
        serde_json::from_value(v.get("writes").cloned().unwrap_or(Value::Null)).map_err(|e| {
            error(
                StatusCode::BAD_REQUEST,
                "bad_batch",
                format!("`writes` is a list of writes: {e}"),
            )
        })
    }) {
        Ok(w) => w,
        Err(r) => return r,
    };
    with(s, method, headers, app, move |s, c, m| {
        let mut conn = s.store.conn();
        let tx = match conn.transaction() {
            Ok(tx) => tx,
            Err(e) => return internal(e),
        };
        let done = match docs::batch(&tx, m, c, &writes, now_ms()) {
            Ok(Ok(done)) => done,
            Ok(Err((at, d))) => {
                let mut r = denied(d);
                r.headers_mut().insert("x-yen-batch-at", at.into());
                return r;
            }
            Err(e) => return internal(e),
        };
        let told = batch_changes(&tx, m, &writes, &done);
        match told.and_then(|t| commit(s, tx, t)) {
            Ok(()) => ok(json!({"done": done})),
            Err(r) => r,
        }
    })
    .await
}

pub async fn share(
    State(s): State<Arc<App>>,
    Path((app, coll, id)): Path<(String, String, String)>,
    method: Method,
    headers: HeaderMap,
    raw: Bytes,
) -> Response {
    let to: Shares = match body(&raw).and_then(|v| {
        serde_json::from_value(v)
            .map_err(|e| error(StatusCode::BAD_REQUEST, "bad_shares", e.to_string()))
    }) {
        Ok(t) => t,
        Err(r) => return r,
    };
    with(s, method, headers, app, move |s, c, m| {
        let mut conn = s.store.conn();
        let tx = match conn.transaction() {
            Ok(tx) => tx,
            Err(e) => return internal(e),
        };
        // Those it leaves hear of it too, and drop it.
        let before = match changed(&tx, m, &coll, &id, Op::Updated) {
            Ok(b) => b.audience,
            Err(r) => return r,
        };
        match docs::share(&tx, m, &coll, c, &id, &to) {
            Ok(Ok(())) => {}
            Ok(Err(d)) => return denied(d),
            Err(e) => return internal(e),
        }
        let told = changed(&tx, m, &coll, &id, Op::Updated).map(|after| Changed {
            audience: either(before, after.audience.clone()),
            ..after
        });
        match told.and_then(|t| commit(s, tx, vec![t])) {
            Ok(()) => ok(&to),
            Err(r) => r,
        }
    })
    .await
}

/// `GET /api/v1/apps`: the apps the caller may open, in the landing page's
/// order.
pub async fn apps(State(s): State<Arc<App>>, method: Method, headers: HeaderMap) -> Response {
    let caller = match active(&s, &method, &headers).await {
        Ok(c) => c,
        Err(r) => return r,
    };
    tokio::task::spawn_blocking(move || {
        let mut out = Vec::new();
        for m in s.apps.all() {
            match may_open(&s.store, &m, &caller) {
                Ok(true) => {}
                Ok(false) => continue,
                Err(e) => return internal(e),
            }
            let access = match access_of(&s.store, &m.id) {
                Ok(a) => a,
                Err(e) => return internal(e),
            };
            let place = (access.position.unwrap_or(i64::MAX), m.name.clone());
            let listed = json!({
                "id": m.id,
                "name": m.name,
                "description": m.description,
                "status": m.status,
                "private": access.private,
                "tile": tile_url(&m),
                "accent": m.theme.accent.as_ref().map(ToString::to_string),
            });
            out.push((place, listed));
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        let apps: Vec<Value> = out.into_iter().map(|(_, v)| v).collect();
        ok(json!({"apps": apps}))
    })
    .await
    .unwrap_or_else(internal)
}

/// `GET /api/v1/apps/{app}`: what the framework builds the app's pages
/// from — its collections, their fields, and whether the caller may add to
/// each.
pub async fn describe(
    State(s): State<Arc<App>>,
    Path(app): Path<String>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    with(s, method, headers, app, move |_, c, m| {
        let collections: Map<String, Value> = m
            .collections
            .iter()
            .map(|(name, coll)| {
                let may_add = coll.scope == Scope::Private || admits(&coll.roles.write, c);
                let described = json!({
                    "scope": coll.scope,
                    "fields": coll.fields,
                    "search": coll.search,
                    "may_add": may_add,
                });
                (name.clone(), described)
            })
            .collect();
        ok(json!({
            "id": m.id,
            "name": m.name,
            "description": m.description,
            "status": m.status,
            "collections": collections,
        }))
    })
    .await
}
