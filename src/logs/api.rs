//! The site as a log source (`candle/docs/mcp_file_tools.md` §6.1), read by
//! the connector's log tools through the gateway:
//!
//! - `GET /v1/logs?grep&level&since&limit&source` → `{lines, matched, complete}`
//! - `GET /v1/logs/sources` → `{sources: [{name, lines, newest}]}`
//!
//! An admin's only.

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use regex::Regex;
use serde::Deserialize;
use serde_json::json;

use super::{Level, Search};
use crate::accounts::identity::identify;
use crate::app::App;

/// Lines answered when a search names no `limit`, and the most.
const DEFAULT_LIMIT: usize = 100;
const MAX_LIMIT: usize = 5000;

#[derive(Deserialize, Default)]
pub struct LogQuery {
    #[serde(default)]
    grep: Option<String>,
    #[serde(default)]
    level: Option<String>,
    #[serde(default)]
    since: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    source: Option<String>,
}

fn refused(status: StatusCode, detail: impl Into<String>) -> Response {
    (
        status,
        Json(json!({"error": "refused", "detail": detail.into()})),
    )
        .into_response()
}

/// `None` when the caller is an admin, else the refusal.
fn admin_only(app: &App, headers: &HeaderMap) -> Option<Response> {
    match identify(headers) {
        Some(id) if app.cfg.is_admin(&id.email) => None,
        Some(_) => Some(refused(
            StatusCode::FORBIDDEN,
            "the site's logs are an admin's",
        )),
        None => Some(refused(StatusCode::UNAUTHORIZED, "sign in as an admin")),
    }
}

/// The search a query asks for, or why it is not one.
pub fn search_of(q: LogQuery) -> Result<Search, String> {
    let grep = q
        .grep
        .filter(|g| !g.is_empty())
        .map(|g| Regex::new(&g).map_err(|e| format!("`grep` is not a regular expression: {e}")))
        .transpose()?;
    let level = match q.level {
        Some(l) => Some(Level::parse(&l).ok_or_else(|| format!("no level `{l}`"))?),
        None => None,
    };
    Ok(Search {
        grep,
        level,
        source: q.source.filter(|s| !s.is_empty()),
        since: q.since.filter(|s| !s.is_empty()),
        limit: q.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT),
    })
}

pub async fn logs(
    State(app): State<Arc<App>>,
    headers: HeaderMap,
    Query(q): Query<LogQuery>,
) -> Response {
    if let Some(no) = admin_only(&app, &headers) {
        return no;
    }
    let search = match search_of(q) {
        Ok(s) => s,
        Err(e) => return refused(StatusCode::BAD_REQUEST, e),
    };
    let logs = app.logs.clone();
    match tokio::task::spawn_blocking(move || logs.search(&search)).await {
        Ok(Ok(found)) => Json(found).into_response(),
        Ok(Err(e)) => refused(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        Err(e) => refused(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub async fn sources(State(app): State<Arc<App>>, headers: HeaderMap) -> Response {
    if let Some(no) = admin_only(&app, &headers) {
        return no;
    }
    let logs = app.logs.clone();
    match tokio::task::spawn_blocking(move || logs.sources()).await {
        Ok(Ok(rows)) => Json(json!({"sources": rows})).into_response(),
        Ok(Err(e)) => refused(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        Err(e) => refused(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_query_becomes_a_search_or_says_why_not() {
        let s = search_of(LogQuery {
            grep: Some("Type(Error)".into()),
            level: Some("warn".into()),
            limit: Some(99_999),
            source: Some("yenbot/recipes".into()),
            ..LogQuery::default()
        })
        .unwrap();
        assert_eq!(
            (s.level, s.limit, s.source.as_deref()),
            (Some(Level::Warn), MAX_LIMIT, Some("yenbot/recipes"))
        );
        assert!(s.grep.unwrap().is_match("TypeError"));
        assert_eq!(search_of(LogQuery::default()).unwrap().limit, DEFAULT_LIMIT);
        assert!(search_of(LogQuery {
            grep: Some("(".into()),
            ..LogQuery::default()
        })
        .is_err());
        assert!(search_of(LogQuery {
            level: Some("LOUD".into()),
            ..LogQuery::default()
        })
        .is_err());
    }
}
