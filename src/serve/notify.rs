//! Notifications over the API (§6.1, §11): the site's push key, a device
//! subscribing and unsubscribing, the account's inbox and marking it read,
//! muting an app, and a feedback note to the admins.

use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::Response;
use serde::Deserialize;
use serde_json::{json, Value};

use super::data::opened;
use super::reply::{active, body, error, internal, ok, Args};
use crate::accounts::caller::Caller;
use crate::accounts::now_ms;
use crate::app::App;
use crate::lifecycle::feedback::keep;
use crate::notify::deliver::{deliver, Origin, ADMINS};
use crate::notify::devices::{self, Subscription};
use crate::notify::inbox::{self, Message};
use crate::notify::vapid::is_push_service;

/// The most notifications one call answers.
const MAX_ITEMS: usize = 100;

/// The longest a feedback note may be, in characters.
const NOTE_MAX: usize = 2_000;

/// The body as `T`, or a 400 naming `what` it should be.
fn shaped<T: for<'de> Deserialize<'de>>(raw: &[u8], what: &str) -> Result<T, Response> {
    serde_json::from_value(body(raw)?).map_err(|e| {
        error(
            StatusCode::BAD_REQUEST,
            "bad_request",
            format!("the body is {what}: {e}"),
        )
    })
}

/// Run `work` on the blocking pool for the active caller.
async fn as_caller<F>(s: Arc<App>, method: Method, headers: HeaderMap, work: F) -> Response
where
    F: FnOnce(&App, &Caller) -> Response + Send + 'static,
{
    match active(&s, &method, &headers).await {
        Ok(c) => tokio::task::spawn_blocking(move || work(&s, &c))
            .await
            .unwrap_or_else(internal),
        Err(r) => r,
    }
}

/// `GET /api/v1/push/key`: what a device subscribes with.
pub async fn key(State(s): State<Arc<App>>) -> Response {
    ok(json!({"key": s.vapid.public()}))
}

/// `POST /api/v1/push/subscribe`: this device, for the caller.
pub async fn subscribe(
    State(s): State<Arc<App>>,
    method: Method,
    headers: HeaderMap,
    raw: Bytes,
) -> Response {
    let sub: Subscription = match shaped(
        &raw,
        "a push subscription, `{endpoint, keys: {p256dh, auth}}`",
    ) {
        Ok(v) => v,
        Err(r) => return r,
    };
    if !is_push_service(&sub.endpoint) {
        return error(
            StatusCode::BAD_REQUEST,
            "bad_endpoint",
            "a push endpoint is a browser's push service: Google's, Mozilla's, Apple's or Microsoft's",
        );
    }
    as_caller(s, method, headers, move |s, c| {
        match devices::subscribe(&s.store.conn(), c.id(), &sub, now_ms()) {
            Ok(()) => ok(json!({"subscribed": true})),
            Err(e) => internal(e),
        }
    })
    .await
}

#[derive(Deserialize)]
struct Endpoint {
    endpoint: String,
}

/// `DELETE /api/v1/push/subscribe`: forget this device.
pub async fn unsubscribe(
    State(s): State<Arc<App>>,
    method: Method,
    headers: HeaderMap,
    raw: Bytes,
) -> Response {
    let e: Endpoint = match shaped(&raw, "`{endpoint}`") {
        Ok(v) => v,
        Err(r) => return r,
    };
    as_caller(s, method, headers, move |s, c| {
        match devices::unsubscribe(&s.store.conn(), c.id(), &e.endpoint) {
            Ok(n) => ok(json!({"unsubscribed": n > 0})),
            Err(e) => internal(e),
        }
    })
    .await
}

#[derive(Debug, Default, Deserialize)]
pub struct InboxQuery {
    #[serde(default)]
    limit: Option<usize>,
}

/// `GET /api/v1/inbox`: the caller's notifications, newest first, how many
/// are unread, and the apps they muted.
pub async fn list(
    State(s): State<Arc<App>>,
    Args(q): Args<InboxQuery>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    as_caller(s, method, headers, move |s, c| {
        let conn = s.store.conn();
        let limit = q.limit.unwrap_or(30).clamp(1, MAX_ITEMS);
        let listed = inbox::list(&conn, c.id(), limit);
        let muted = devices::muted(&conn, c.id());
        match (listed, muted) {
            (Ok((items, unread)), Ok(muted)) => {
                ok(json!({"items": items, "unread": unread, "muted": muted}))
            }
            (Err(e), _) | (_, Err(e)) => internal(e),
        }
    })
    .await
}

/// `POST /api/v1/inbox/{id}/read`: one notification read, or `all`.
pub async fn read(
    State(s): State<Arc<App>>,
    Path(id): Path<String>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    as_caller(s, method, headers, move |s, c| {
        match inbox::read(&s.store.conn(), c.id(), &id, now_ms()) {
            Ok(n) => ok(json!({"read": n})),
            Err(e) => internal(e),
        }
    })
    .await
}

#[derive(Deserialize)]
struct Mute {
    muted: bool,
}

/// `PUT /api/v1/notify/{app}/mute`: the app's pushes stopped, or started
/// again, for the caller; its inbox keeps every one.
pub async fn mute(
    State(s): State<Arc<App>>,
    Path(app): Path<String>,
    method: Method,
    headers: HeaderMap,
    raw: Bytes,
) -> Response {
    let m: Mute = match shaped(&raw, "`{muted}`") {
        Ok(v) => v,
        Err(r) => return r,
    };
    as_caller(s, method, headers, move |s, c| {
        if let Err(r) = opened(s, &app, c) {
            return r;
        }
        match devices::mute(&s.store.conn(), c.id(), &app, m.muted) {
            Ok(()) => ok(json!({"app": app, "muted": m.muted})),
            Err(e) => internal(e),
        }
    })
    .await
}

/// `POST /api/v1/feedback/{app}`: a note from an app's own menu, into the
/// admins' inboxes.
pub async fn feedback(
    State(s): State<Arc<App>>,
    Path(app): Path<String>,
    method: Method,
    headers: HeaderMap,
    raw: Bytes,
) -> Response {
    let text = match body(&raw).map(|v| {
        v.get("text")
            .and_then(Value::as_str)
            .map(str::trim)
            .map(str::to_string)
    }) {
        Ok(Some(t)) if !t.is_empty() => t.chars().take(NOTE_MAX).collect::<String>(),
        Ok(_) => {
            return error(
                StatusCode::BAD_REQUEST,
                "bad_request",
                "a note has its `text`",
            )
        }
        Err(r) => return r,
    };
    as_caller(s, method, headers, move |s, c| {
        let m = match opened(s, &app, c) {
            Ok(m) => m,
            Err(r) => return r,
        };
        if let Err(e) = keep(&s.store.conn(), &m.id, c.id(), &text, now_ms()) {
            return internal(e);
        }
        let note = Message {
            title: format!("{} on {}", c.account.name, m.name),
            body: text,
            open: Some(format!("/a/{}/", m.id)),
        };
        let origin = Origin {
            store: &s.store,
            cfg: &s.cfg,
            pusher: &s.pusher,
            app: &m,
        };
        match deliver(&origin, ADMINS, &note, now_ms()) {
            Ok(Ok(done)) => ok(json!({"sent": done})),
            Ok(Err(why)) => error(StatusCode::CONFLICT, "no_one", why),
            Err(e) => internal(e),
        }
    })
    .await
}
