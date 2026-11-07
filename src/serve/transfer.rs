//! `/api/v1/export/{app}` and `/api/v1/import/{app}` (§20.1): an app's
//! records out as JSON or a spreadsheet's CSV, and back in — as the caller,
//! or, for an admin, every account's on the way out.

use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use super::data::with;
use super::reply::{body, denied, error, internal, ok, Args};
use crate::accounts::now_ms;
use crate::app::App;
use crate::data::changes::Feed;
use crate::data::docs::Denied;
use crate::data::export::{as_csv, as_json};
use crate::data::import::{from_csv, from_json, Stopped};
use crate::data::validate::Refused;

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transfer {
    #[serde(default)]
    format: Option<String>,
    #[serde(default)]
    coll: Option<String>,
}

/// Whether the transfer is CSV — which names one collection — or JSON.
fn csv_of(t: &Transfer) -> Result<Option<String>, Response> {
    match (t.format.as_deref().unwrap_or("json"), &t.coll) {
        ("json", _) => Ok(None),
        ("csv", Some(coll)) => Ok(Some(coll.clone())),
        ("csv", None) => Err(error(
            StatusCode::BAD_REQUEST,
            "bad_request",
            "a CSV is one collection: name it with `?coll=`",
        )),
        (other, _) => Err(error(
            StatusCode::BAD_REQUEST,
            "bad_request",
            format!("`{other}` is not a format: `json` or `csv`"),
        )),
    }
}

fn attachment(bytes: String, kind: &'static str, name: &str) -> Response {
    let mut res = Body::from(bytes).into_response();
    let h = res.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_static(kind));
    if let Ok(v) = HeaderValue::from_str(&format!("attachment; filename=\"{name}\"")) {
        h.insert(header::CONTENT_DISPOSITION, v);
    }
    res
}

/// `GET /api/v1/export/{app}`.
pub async fn export(
    State(s): State<Arc<App>>,
    Path(app): Path<String>,
    Args(t): Args<Transfer>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    let csv = match csv_of(&t) {
        Ok(c) => c,
        Err(r) => return r,
    };
    with(s, method, headers, app, move |s, c, m| {
        let conn = s.store.conn();
        match &csv {
            None => match as_json(&conn, m, c, c.is_admin()) {
                Ok(Ok(v)) => {
                    attachment(v.to_string(), "application/json", &format!("{}.json", m.id))
                }
                Ok(Err(d)) => denied(d),
                Err(e) => internal(e),
            },
            Some(coll) => match as_csv(&conn, m, coll, c, c.is_admin()) {
                Ok(Ok(text)) => attachment(
                    text,
                    "text/csv; charset=utf-8",
                    &format!("{}-{coll}.csv", m.id),
                ),
                Ok(Err(d)) => denied(d),
                Err(e) => internal(e),
            },
        }
    })
    .await
}

/// An import stopped: which record, and why.
fn stopped(s: Stopped) -> Response {
    let at = if s.row == 0 {
        s.coll.clone()
    } else {
        format!("record {} of {}", s.row, s.coll)
    };
    match s.denied {
        Denied::Invalid(Refused { field, reason }) => {
            let body = json!({
                "error": "invalid",
                "detail": format!("{at}: {field} {reason}"),
                "field": field,
                "coll": s.coll,
                "row": s.row,
            });
            (StatusCode::UNPROCESSABLE_ENTITY, Json(body)).into_response()
        }
        Denied::Bad(why) => error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "bad_import",
            format!("{at}: {why}"),
        ),
        other => denied(other),
    }
}

/// `POST /api/v1/import/{app}`.
pub async fn import(
    State(s): State<Arc<App>>,
    Path(app): Path<String>,
    Args(t): Args<Transfer>,
    method: Method,
    headers: HeaderMap,
    raw: Bytes,
) -> Response {
    let csv = match csv_of(&t) {
        Ok(c) => c,
        Err(r) => return r,
    };
    let json_body = match &csv {
        None => match body(&raw) {
            Ok(v) => Some(v),
            Err(r) => return r,
        },
        Some(_) => None,
    };
    with(s, method, headers, app, move |s, c, m| {
        let text = String::from_utf8_lossy(&raw).into_owned();
        let mut conn = s.store.conn();
        let tx = match conn.transaction() {
            Ok(tx) => tx,
            Err(e) => return internal(e),
        };
        let now = now_ms();
        let made = match (&csv, &json_body) {
            (Some(coll), _) => from_csv(&tx, m, c, coll, &text, now),
            (None, Some(v)) => from_json(&tx, m, c, v, now),
            (None, None) => return internal("an import with no body read"),
        };
        match made {
            Ok(Ok(made)) => {
                if let Err(e) = tx.commit() {
                    return internal(e);
                }
                s.changes.send(Feed::Reset(Some(m.id.clone())));
                ok(json!({"app": m.id, "created": made}))
            }
            Ok(Err(stop)) => stopped(stop),
            Err(e) => internal(e),
        }
    })
    .await
}
