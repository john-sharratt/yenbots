//! What the admin page reads of the apps over time (§6.2, §13.2, §20): how
//! each is used, what browsers reported, the notes sent from them, and the
//! archive — with archiving an app and restoring one.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::Response;
use serde::Deserialize;
use serde_json::json;

use super::admin::{as_admin, no_app};
use super::reply::{error, internal, ok, Args};
use crate::accounts::now_ms;
use crate::app::App;
use crate::jobs::scheduler::sync_all;
use crate::lifecycle::archive::{self, Reason};
use crate::lifecycle::feedback;
use crate::lifecycle::usage::{day_of, summed, By};
use crate::logs::reported::reported;

/// The longest lists answered.
const MAX_LISTED: usize = 500;

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsageQuery {
    #[serde(default)]
    app: Option<String>,
    #[serde(default)]
    since: Option<String>,
    #[serde(default)]
    by: Option<String>,
}

/// Whether `s` is a day, `YYYY-MM-DD`.
fn is_day(s: &str) -> bool {
    s.len() == 10
        && s.chars().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                c == '-'
            } else {
                c.is_ascii_digit()
            }
        })
}

/// `GET /admin/usage`: opens, accounts and AI calls by app and day — or by
/// account — over the last four weeks unless `since` says otherwise.
pub async fn usage(
    State(s): State<Arc<App>>,
    Args(q): Args<UsageQuery>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    let Some(by) = By::parse(q.by.as_deref().unwrap_or("day")) else {
        return error(
            StatusCode::BAD_REQUEST,
            "bad_request",
            "`by` is `day` or `account`",
        );
    };
    if let Some(since) = q.since.as_deref().filter(|d| !is_day(d)) {
        return error(
            StatusCode::BAD_REQUEST,
            "bad_request",
            format!("`{since}` is not a day, `YYYY-MM-DD`"),
        );
    }
    as_admin(s, method, headers, move |s, _| {
        let tz = s.cfg.zone();
        let since = q
            .since
            .clone()
            .unwrap_or_else(|| day_of(now_ms() - 27 * 86_400_000, &tz));
        match summed(&s.store.conn(), q.app.as_deref(), &since, by, &tz) {
            Ok(rows) => {
                ok(json!({"since": since, "by": q.by.as_deref().unwrap_or("day"), "usage": rows}))
            }
            Err(e) => internal(e),
        }
    })
    .await
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListQuery {
    #[serde(default)]
    app: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

/// `GET /admin/errors`: what browsers reported, newest first.
pub async fn errors(
    State(s): State<Arc<App>>,
    Args(q): Args<ListQuery>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    as_admin(s, method, headers, move |s, _| {
        let limit = q.limit.unwrap_or(100).clamp(1, MAX_LISTED);
        match reported(&s.store.conn(), q.app.as_deref(), limit) {
            Ok(e) => ok(json!({"errors": e})),
            Err(e) => internal(e),
        }
    })
    .await
}

/// `GET /admin/feedback`: the notes sent from the apps, newest first.
pub async fn notes(
    State(s): State<Arc<App>>,
    Args(q): Args<ListQuery>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    as_admin(s, method, headers, move |s, _| {
        let limit = q.limit.unwrap_or(100).clamp(1, MAX_LISTED);
        match feedback::list(&s.store.conn(), q.app.as_deref(), limit) {
            Ok(n) => ok(json!({"feedback": n})),
            Err(e) => internal(e),
        }
    })
    .await
}

/// `GET /admin/archive`.
pub async fn archived(State(s): State<Arc<App>>, method: Method, headers: HeaderMap) -> Response {
    as_admin(s, method, headers, |s, _| {
        match archive::list(&s.store.conn()) {
            Ok(a) => ok(json!({"archived": a})),
            Err(e) => internal(e),
        }
    })
    .await
}

/// `POST /admin/apps/{id}/archive`: hide an app, its jobs held, until it
/// is restored.
pub async fn archive_app(
    State(s): State<Arc<App>>,
    Path(id): Path<String>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    as_admin(s, method, headers, move |s, _| {
        if s.apps.get(&id).is_none() {
            return no_app(&id);
        }
        let done = archive::archive(&s.store.conn(), &id, Reason::Admin, now_ms());
        match done {
            Ok(_) => {
                s.apps.reload();
                ok(json!({"app": id, "archived": true}))
            }
            Err(e) => internal(e),
        }
    })
    .await
}

/// `POST /admin/apps/{id}/restore`: serve an archived app again — once its
/// folder is there.
pub async fn restore_app(
    State(s): State<Arc<App>>,
    Path(id): Path<String>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    as_admin(s, method, headers, move |s, _| {
        if !s.apps.present().contains(&id) {
            return error(
                StatusCode::CONFLICT,
                "no_folder",
                format!(
                    "`apps/{id}/` is not in the repository: bring it back from the commit before \
                     it was removed, and the app is restored as it lands"
                ),
            );
        }
        let done = archive::restore(&s.store.conn(), &id, now_ms());
        match done {
            Ok(false) => error(
                StatusCode::NOT_FOUND,
                "not_archived",
                format!("`{id}` is not archived"),
            ),
            Ok(true) => {
                s.apps.reload();
                if let Err(e) = sync_all(s, now_ms()) {
                    return internal(e);
                }
                s.jobs.wake();
                ok(json!({"app": id, "restored": true}))
            }
            Err(e) => internal(e),
        }
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_day_is_written_in_full() {
        assert!(is_day("2026-11-07"));
        assert!(!is_day("2026-11-7"));
        assert!(!is_day("2026/11/07"));
        assert!(!is_day("tomorrow"));
    }
}
