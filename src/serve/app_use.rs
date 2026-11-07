//! An account's use of an app (§6.1, §20.3): `POST /api/v1/opens/{app}`,
//! which the framework posts once per page load, and the account's own
//! settings object for the app, `GET/PUT /api/v1/settings/{app}`.

use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::Response;
use serde_json::json;

use super::data::with;
use super::reply::{body, error, internal, ok};
use crate::accounts::now_ms;
use crate::app::App;
use crate::lifecycle::settings;
use crate::lifecycle::usage::opened;

/// `POST /api/v1/opens/{app}`.
pub async fn open(
    State(s): State<Arc<App>>,
    Path(app): Path<String>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    with(s, method, headers, app, |s, c, m| {
        match opened(&s.store.conn(), &m.id, c.id(), now_ms(), &s.cfg.zone()) {
            Ok(()) => ok(json!({"app": m.id, "opened": true})),
            Err(e) => internal(e),
        }
    })
    .await
}

/// `GET /api/v1/settings/{app}`.
pub async fn get(
    State(s): State<Arc<App>>,
    Path(app): Path<String>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    with(s, method, headers, app, |s, c, m| {
        match settings::get(&s.store.conn(), &m.id, c.id()) {
            Ok(v) => ok(json!({"app": m.id, "settings": v})),
            Err(e) => internal(e),
        }
    })
    .await
}

/// `PUT /api/v1/settings/{app}`: the whole object, replacing what it was.
pub async fn put(
    State(s): State<Arc<App>>,
    Path(app): Path<String>,
    method: Method,
    headers: HeaderMap,
    raw: Bytes,
) -> Response {
    let value = match body(&raw) {
        Ok(v) => v,
        Err(r) => return r,
    };
    with(s, method, headers, app, move |s, c, m| match settings::put(
        &s.store.conn(),
        &m.id,
        c.id(),
        &value,
        now_ms(),
    ) {
        Ok(()) => ok(json!({"app": m.id, "settings": value})),
        Err(e) => error(StatusCode::BAD_REQUEST, "bad_settings", e.to_string()),
    })
    .await
}
