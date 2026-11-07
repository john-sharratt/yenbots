//! `/hooks/{app}/{name}` (§19.2): a service calling in runs the app's
//! `workflows/hooks/<name>.js`, as the app, handed the request —
//! `{method, query, headers, text, json}`. The call proves itself with the
//! hook's token or a signature of its body (`integrations::hooks`); one that
//! does neither is refused, and the refusal is written to the app's log.
//! The workflow's answer is the call's: `{ok: true, value}`, or 500.

use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::Result;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};

use super::reply::{error, internal, ok, Args};
use crate::accounts::caller::Caller;
use crate::accounts::now_ms;
use crate::app::App;
use crate::integrations::hooks::{signature_holds, signing_secret, token_holds, workflow_of};
use crate::integrations::secrets;
use crate::jobs::runner::{run_now, Now};
use crate::logs::browser::source;
use crate::logs::{Level, Line};
use crate::workflows::sources;

/// The signature headers read, the site's own first.
const SIGNATURES: [&str; 2] = ["x-yen-signature", "x-hub-signature-256"];

/// The headers a hook is never handed.
fn withheld(name: &str) -> bool {
    name == "authorization" || name == "cookie" || name.starts_with("x-tokera-")
}

fn no_hook() -> Response {
    error(StatusCode::NOT_FOUND, "no_hook", "there is no such hook")
}

/// The token the call carries: `?token=`, or `Authorization: Bearer`.
fn token_of<'a>(query: &'a BTreeMap<String, String>, headers: &'a HeaderMap) -> Option<&'a str> {
    query.get("token").map(String::as_str).or_else(|| {
        headers
            .get(header::AUTHORIZATION)?
            .to_str()
            .ok()?
            .strip_prefix("Bearer ")
    })
}

/// Whether the call proves itself for `app`'s hook `name`.
fn proven(
    s: &App,
    app: &str,
    name: &str,
    query: &BTreeMap<String, String>,
    headers: &HeaderMap,
    body: &[u8],
) -> Result<bool> {
    let conn = s.store.conn();
    if let Some(token) = token_of(query, headers) {
        return token_holds(&conn, app, name, token);
    }
    let Some(sig) = SIGNATURES
        .iter()
        .find_map(|h| headers.get(*h)?.to_str().ok())
    else {
        return Ok(false);
    };
    Ok(match secrets::get(&conn, app, &signing_secret(name))? {
        Some(key) => signature_holds(key.as_bytes(), body, sig),
        None => false,
    })
}

/// What the workflow is handed.
fn request_of(
    method: &Method,
    query: BTreeMap<String, String>,
    headers: &HeaderMap,
    body: &[u8],
) -> Value {
    let mut query = query;
    query.remove("token");
    let shown: BTreeMap<String, String> = headers
        .iter()
        .filter(|(k, _)| !withheld(k.as_str()))
        .filter_map(|(k, v)| Some((k.as_str().to_string(), v.to_str().ok()?.to_string())))
        .collect();
    let text = String::from_utf8_lossy(body).into_owned();
    let parsed = serde_json::from_str::<Value>(&text).unwrap_or(Value::Null);
    json!({
        "method": method.as_str(),
        "query": query,
        "headers": shown,
        "text": text,
        "json": parsed,
    })
}

/// `GET|POST|PUT /hooks/{app}/{name}`.
pub async fn hook(
    State(s): State<Arc<App>>,
    Path((app, name)): Path<(String, String)>,
    Args(query): Args<BTreeMap<String, String>>,
    method: Method,
    headers: HeaderMap,
    raw: Bytes,
) -> Response {
    tokio::task::spawn_blocking(move || {
        let Some(m) = s.apps.get(&app) else {
            return no_hook();
        };
        let workflow = workflow_of(&name);
        let src = match sources::of(&s.apps, &m.id) {
            Ok(src) if src.has(&workflow) => src,
            Ok(_) => return no_hook(),
            Err(e) => return internal(e),
        };
        match proven(&s, &m.id, &name, &query, &headers, &raw) {
            Ok(true) => {}
            Ok(false) => {
                let refused = Line {
                    at_ms: now_ms(),
                    level: Level::Warn,
                    source: source(&m.id),
                    target: "hook".into(),
                    message: format!(
                        "hook `{name}`: a {method} call with neither its token nor a signature was refused"
                    ),
                };
                if let Err(e) = s.logs.write(&[refused]) {
                    return internal(e);
                }
                return error(
                    StatusCode::UNAUTHORIZED,
                    "unsigned",
                    "a hook is called with its token or a signature",
                );
            }
            Err(e) => return internal(e),
        }
        let args = request_of(&method, query, &headers, &raw);
        let caller = Caller::app(&m.id, &m.name);
        let now = Now {
            m,
            sources: src,
            workflow: &workflow,
        };
        match run_now(&s, now, args, caller) {
            Ok(outcome) => match outcome.answer {
                Ok(value) => ok(json!({"ok": true, "value": value})),
                Err(f) => {
                    let body = json!({"error": "hook_failed", "detail": f.to_string()});
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(body)).into_response()
                }
            },
            Err(e) => internal(e),
        }
    })
    .await
    .unwrap_or_else(internal)
}
