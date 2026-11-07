//! `/api/v1/admin/secrets/…` and `/api/v1/admin/hooks/…` (§6.2, §19): an
//! app's integration keys — set, removed, listed by name, never answered —
//! and its inbound hooks, each with the token it is called with, made and
//! shown once.

use std::sync::Arc;

use anyhow::Result;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::Response;
use serde_json::json;

use super::admin::{as_admin, field, no_app};
use super::reply::{error, internal, ok};
use crate::accounts::now_ms;
use crate::app::App;
use crate::integrations::hooks::{self, workflow_of};
use crate::integrations::secrets;
use crate::workflows::sources;

/// `GET /admin/secrets/{app}`.
pub async fn secrets_list(
    State(s): State<Arc<App>>,
    Path(app): Path<String>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    as_admin(s, method, headers, move |s, _| {
        if s.apps.get(&app).is_none() {
            return no_app(&app);
        }
        match secrets::names(&s.store.conn(), &app) {
            Ok(held) => ok(json!({"app": app, "secrets": held})),
            Err(e) => internal(e),
        }
    })
    .await
}

/// `PUT /admin/secrets/{app}/{name}`.
pub async fn secrets_set(
    State(s): State<Arc<App>>,
    Path((app, name)): Path<(String, String)>,
    method: Method,
    headers: HeaderMap,
    raw: Bytes,
) -> Response {
    let value: String = match field(&raw, "value") {
        Ok(v) => v,
        Err(r) => return r,
    };
    as_admin(s, method, headers, move |s, _| {
        if s.apps.get(&app).is_none() {
            return no_app(&app);
        }
        match secrets::set(&s.store.conn(), &app, &name, &value, now_ms()) {
            Ok(()) => ok(json!({"app": app, "name": name, "set": true})),
            Err(e) => error(StatusCode::BAD_REQUEST, "bad_secret", e.to_string()),
        }
    })
    .await
}

/// `DELETE /admin/secrets/{app}/{name}`.
pub async fn secrets_remove(
    State(s): State<Arc<App>>,
    Path((app, name)): Path<(String, String)>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    as_admin(s, method, headers, move |s, _| {
        match secrets::remove(&s.store.conn(), &app, &name) {
            Ok(true) => ok(json!({"app": app, "removed": name})),
            Ok(false) => error(
                StatusCode::NOT_FOUND,
                "not_found",
                format!("{app} has no secret `{name}`"),
            ),
            Err(e) => internal(e),
        }
    })
    .await
}

/// The hooks app `app` has — its files under `workflows/hooks/` — by name.
fn hooks_of(s: &App, app: &str) -> Result<Vec<String>> {
    let src = sources::of(&s.apps, app)?;
    let under = format!("{}hooks/", src.dir);
    Ok(src
        .files
        .keys()
        .filter_map(|p| p.strip_prefix(&under)?.strip_suffix(".js"))
        .filter(|n| !n.contains('/'))
        .map(str::to_string)
        .collect())
}

/// `GET /admin/hooks/{app}`: each hook, its address, and whether it has a
/// token.
pub async fn hooks_list(
    State(s): State<Arc<App>>,
    Path(app): Path<String>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    as_admin(s, method, headers, move |s, _| {
        if s.apps.get(&app).is_none() {
            return no_app(&app);
        }
        let (names, tokens) = match (hooks_of(s, &app), hooks::list(&s.store.conn(), &app)) {
            (Ok(n), Ok(t)) => (n, t),
            (Err(e), _) | (_, Err(e)) => return internal(e),
        };
        let shown: Vec<_> = names
            .iter()
            .map(|n| {
                let made = tokens.iter().find(|t| &t.hook == n).map(|t| t.created_ms);
                json!({
                    "hook": n,
                    "url": format!("/hooks/{app}/{n}"),
                    "token_made_ms": made,
                    "signing_secret": hooks::signing_secret(n),
                })
            })
            .collect();
        ok(json!({"app": app, "hooks": shown}))
    })
    .await
}

/// `POST /admin/hooks/{app}/{name}`: a new token for the hook, replacing
/// the one it had; shown once.
pub async fn hooks_token(
    State(s): State<Arc<App>>,
    Path((app, name)): Path<(String, String)>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    as_admin(s, method, headers, move |s, _| {
        if s.apps.get(&app).is_none() {
            return no_app(&app);
        }
        match sources::of(&s.apps, &app) {
            Ok(src) if src.has(&workflow_of(&name)) => {}
            Ok(_) => {
                return error(
                    StatusCode::NOT_FOUND,
                    "no_hook",
                    format!("{app} has no hook `{name}`: add `workflows/hooks/{name}.js`"),
                )
            }
            Err(e) => return internal(e),
        }
        match hooks::make(&s.store.conn(), &app, &name, now_ms()) {
            Ok(token) => ok(json!({
                "app": app,
                "hook": name,
                "url": format!("/hooks/{app}/{name}?token={token}"),
                "token": token,
                "note": "shown once: give it to the service that calls in",
            })),
            Err(e) => internal(e),
        }
    })
    .await
}

/// `DELETE /admin/hooks/{app}/{name}`: revoke the hook's token.
pub async fn hooks_revoke(
    State(s): State<Arc<App>>,
    Path((app, name)): Path<(String, String)>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    as_admin(s, method, headers, move |s, _| {
        match hooks::revoke(&s.store.conn(), &app, &name) {
            Ok(true) => ok(json!({"app": app, "revoked": name})),
            Ok(false) => error(
                StatusCode::NOT_FOUND,
                "not_found",
                format!("{app}'s hook `{name}` has no token"),
            ),
            Err(e) => internal(e),
        }
    })
    .await
}
