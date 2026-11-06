//! `/api/v1/data/…` and `/api/v1/apps` (§6.1): an app's records, read and
//! written as the caller, and the apps the caller may open.

use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use super::reply::{active, body, denied, error, internal, ok, Args};
use crate::accounts::caller::Caller;
use crate::accounts::now_ms;
use crate::app::App;
use crate::apps::access::{access_of, may_open};
use crate::apps::manifest::Manifest;
use crate::data::docs::{self, Change, Query as DocQuery, Shares, Sum, Write};

fn no_app(id: &str) -> Response {
    error(
        StatusCode::NOT_FOUND,
        "no_app",
        format!("there is no app `{id}`"),
    )
}

/// The app `id`, when the caller may open it. One the caller may not open
/// is answered as one that is not there.
fn opened(state: &App, id: &str, caller: &Caller) -> Result<Arc<Manifest>, Response> {
    let m = state.apps.get(id).ok_or_else(|| no_app(id))?;
    match may_open(&state.store, &m, caller) {
        Ok(true) => Ok(m),
        Ok(false) => Err(no_app(id)),
        Err(e) => Err(internal(e)),
    }
}

/// Run `work` on the blocking pool with the caller and the app.
async fn with<F>(
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
        match docs::create(&tx, m, &coll, c, &data, now_ms()) {
            Ok(Ok(doc)) => match tx.commit() {
                Ok(()) => (StatusCode::CREATED, Json(doc)).into_response(),
                Err(e) => internal(e),
            },
            Ok(Err(d)) => denied(d),
            Err(e) => internal(e),
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
        match docs::update(&tx, m, &coll, c, &id, change, now_ms()) {
            Ok(Ok(doc)) => match tx.commit() {
                Ok(()) => ok(doc),
                Err(e) => internal(e),
            },
            Ok(Err(d)) => denied(d),
            Err(e) => internal(e),
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
        match docs::delete(&tx, m, &coll, c, &id) {
            Ok(Ok(())) => match tx.commit() {
                Ok(()) => ok(json!({"deleted": id})),
                Err(e) => internal(e),
            },
            Ok(Err(d)) => denied(d),
            Err(e) => internal(e),
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
        match docs::batch(&tx, m, c, &writes, now_ms()) {
            Ok(Ok(done)) => match tx.commit() {
                Ok(()) => ok(json!({"done": done})),
                Err(e) => internal(e),
            },
            Ok(Err((at, d))) => {
                let mut r = denied(d);
                r.headers_mut().insert("x-yen-batch-at", at.into());
                r
            }
            Err(e) => internal(e),
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
        match docs::share(&tx, m, &coll, c, &id, &to) {
            Ok(Ok(())) => match tx.commit() {
                Ok(()) => ok(&to),
                Err(e) => internal(e),
            },
            Ok(Err(d)) => denied(d),
            Err(e) => internal(e),
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
