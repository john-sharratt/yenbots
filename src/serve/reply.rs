//! What every API route answers with: JSON, an error as
//! `{"error", "detail", "field"?}` — the shape the framework's forms read —
//! and the caller resolved once per request.

use std::sync::Arc;

use axum::async_trait;
use axum::extract::{FromRequestParts, Query};
use axum::http::request::Parts;
use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

use crate::accounts::caller::{who, Caller, Who};
use crate::app::App;
use crate::data::docs::Denied;

/// The header the framework sends on every call: a browser on another site
/// cannot send it without asking first, so a write without it is refused.
pub const APP_HEADER: &str = "x-yen-app";

pub fn error(status: StatusCode, code: &str, detail: impl Into<String>) -> Response {
    let body = json!({"error": code, "detail": detail.into()});
    (status, Json(body)).into_response()
}

pub fn ok(value: impl serde::Serialize) -> Response {
    Json(value).into_response()
}

pub fn denied(d: Denied) -> Response {
    match d {
        Denied::NoCollection(why) => error(StatusCode::NOT_FOUND, "no_collection", why),
        Denied::NotFound => error(StatusCode::NOT_FOUND, "not_found", "no such record"),
        Denied::Forbidden(why) => error(StatusCode::FORBIDDEN, "forbidden", why),
        Denied::Stale { held } => {
            let body = json!({
                "error": "stale",
                "detail": "the record changed since it was read",
                "version": held,
            });
            (StatusCode::PRECONDITION_FAILED, Json(body)).into_response()
        }
        Denied::Invalid(r) => {
            let body = json!({
                "error": "invalid",
                "detail": format!("{} {}", r.field, r.reason),
                "field": r.field,
                "reason": r.reason,
            });
            (StatusCode::UNPROCESSABLE_ENTITY, Json(body)).into_response()
        }
        Denied::Bad(why) => error(StatusCode::BAD_REQUEST, "bad_request", why),
    }
}

pub fn internal(e: impl std::fmt::Display) -> Response {
    tracing::error!("api: {e}");
    error(StatusCode::INTERNAL_SERVER_ERROR, "internal", e.to_string())
}

/// The active caller of a request, or the answer for one who is not: nobody
/// (401), waiting or blocked (403), or a browser write without the
/// framework's header (403).
pub async fn active(
    app: &Arc<App>,
    method: &Method,
    headers: &HeaderMap,
) -> Result<Caller, Response> {
    let (a, h) = (app.clone(), headers.clone());
    let found = tokio::task::spawn_blocking(move || who(&a.store, &a.cfg, &a.pusher, &h))
        .await
        .map_err(internal)?
        .map_err(internal)?;
    let caller = match found {
        Who::Nobody => {
            return Err(error(
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "sign in first",
            ))
        }
        Who::Waiting(_) => {
            return Err(error(
                StatusCode::FORBIDDEN,
                "not_active",
                "this account is not approved yet",
            ))
        }
        Who::Active(c) => c,
    };
    let writes = !matches!(*method, Method::GET | Method::HEAD);
    if writes && caller.token.is_none() && !headers.contains_key(APP_HEADER) {
        return Err(error(
            StatusCode::FORBIDDEN,
            "no_app_header",
            "writes come from the site's own pages, which send X-Yen-App",
        ));
    }
    Ok(caller)
}

/// The admin caller of a request, or the answer for one who is not.
pub async fn admin(
    app: &Arc<App>,
    method: &Method,
    headers: &HeaderMap,
) -> Result<Caller, Response> {
    let c = active(app, method, headers).await?;
    if c.is_admin() {
        Ok(c)
    } else {
        Err(error(
            StatusCode::FORBIDDEN,
            "forbidden",
            "this is an admin's",
        ))
    }
}

/// A route's query string, refused as the API refuses everything else —
/// `{error, detail}` — rather than as plain text.
pub struct Args<T>(pub T);

#[async_trait]
impl<T, S> FromRequestParts<S> for Args<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Response> {
        match Query::<T>::from_request_parts(parts, state).await {
            Ok(Query(v)) => Ok(Args(v)),
            Err(e) => Err(error(StatusCode::BAD_REQUEST, "bad_query", e.body_text())),
        }
    }
}

/// `value` as JSON, or a 400.
pub fn body(value: &[u8]) -> Result<Value, Response> {
    serde_json::from_slice(value).map_err(|e| {
        error(
            StatusCode::BAD_REQUEST,
            "bad_json",
            format!("the body is not JSON: {e}"),
        )
    })
}
