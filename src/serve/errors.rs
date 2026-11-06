//! `POST /api/v1/errors` (§18.1): what went wrong in a browser, written under
//! the app's log source with the page, the account, the stack and the
//! browser, repeats counted rather than flooding
//! ([`crate::logs::browser`]).
//!
//! Any signed-in account may report, approved or not — a page that fails
//! for a newcomer is a page that fails — but only from the site's own pages
//! (`X-Yen-App`), and only for an app there is.

use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::Response;
use serde_json::json;

use super::reply::{body, error, internal, ok, APP_HEADER};
use crate::accounts::caller::{who, Who};
use crate::accounts::now_ms;
use crate::app::App;
use crate::logs::browser::{line, Report};

pub async fn report(State(s): State<Arc<App>>, headers: HeaderMap, raw: Bytes) -> Response {
    if !headers.contains_key(APP_HEADER) {
        return error(
            StatusCode::FORBIDDEN,
            "no_app_header",
            "errors come from the site's own pages, which send X-Yen-App",
        );
    }
    let report: Report = match body(&raw).and_then(|v| {
        serde_json::from_value(v)
            .map_err(|e| error(StatusCode::BAD_REQUEST, "bad_report", e.to_string()))
    }) {
        Ok(r) => r,
        Err(r) => return r,
    };
    let browser = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("an unnamed browser")
        .to_string();
    tokio::task::spawn_blocking(move || {
        let account = match who(&s.store, &s.cfg, &headers) {
            Ok(Who::Active(c)) => c.account.id,
            Ok(Who::Waiting(a)) => a.id,
            Ok(Who::Nobody) => {
                return error(StatusCode::UNAUTHORIZED, "unauthorized", "sign in first")
            }
            Err(e) => return internal(e),
        };
        if report.app != "_site" && s.apps.get(&report.app).is_none() {
            return error(
                StatusCode::NOT_FOUND,
                "no_app",
                format!("there is no app `{}`", report.app),
            );
        }
        let now = now_ms();
        let key = format!("{}\n{}", report.app, report.message);
        let Some(earlier) = s.repeats.note(&key, now, 1 + report.repeats) else {
            return ok(json!({"noted": "counted"}));
        };
        match s
            .logs
            .write(&[line(&report, &account, &browser, earlier, now)])
        {
            Ok(()) => ok(json!({"noted": "written"})),
            Err(e) => internal(e),
        }
    })
    .await
    .unwrap_or_else(internal)
}
