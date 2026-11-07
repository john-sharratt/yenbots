//! `/api/v1/files/…` (§6.1): a file kept for an app — a photo for a record,
//! a PDF — and each served back, with a picture's thumbnail, to those who
//! may open the app. A file never changes once kept, so it is served to be
//! kept by the browser for a year.

use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;

use super::data::opened;
use super::reply::{active, error, internal, Args};
use crate::accounts::now_ms;
use crate::app::App;

#[derive(Debug, Default, Deserialize)]
pub struct Named {
    #[serde(default)]
    name: Option<String>,
}

/// `POST /api/v1/files/{app}`: keep the body as one of the app's files.
pub async fn upload(
    State(s): State<Arc<App>>,
    Path(app): Path<String>,
    Args(q): Args<Named>,
    method: Method,
    headers: HeaderMap,
    raw: Bytes,
) -> Response {
    let caller = match active(&s, &method, &headers).await {
        Ok(c) => c,
        Err(r) => return r,
    };
    tokio::task::spawn_blocking(move || {
        let m = match opened(&s, &app, &caller) {
            Ok(m) => m,
            Err(r) => return r,
        };
        let name = q.name.unwrap_or_else(|| "file".to_string());
        match s
            .files
            .keep(&s.store.conn(), &m.id, caller.id(), &name, &raw, now_ms())
        {
            Ok(k) => (StatusCode::CREATED, Json(k.shown())).into_response(),
            Err(e) => error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "bad_file",
                format!("{e:#}"),
            ),
        }
    })
    .await
    .unwrap_or_else(internal)
}

async fn serve_file(
    s: Arc<App>,
    app: String,
    id: String,
    thumb: bool,
    method: Method,
    headers: HeaderMap,
) -> Response {
    let caller = match active(&s, &method, &headers).await {
        Ok(c) => c,
        Err(r) => return r,
    };
    tokio::task::spawn_blocking(move || {
        let m = match opened(&s, &app, &caller) {
            Ok(m) => m,
            Err(r) => return r,
        };
        let kept = match s.files.get(&s.store.conn(), &m.id, &id) {
            Ok(Some(k)) if !thumb || k.kind.is_picture() => k,
            Ok(_) => {
                return error(
                    StatusCode::NOT_FOUND,
                    "no_file",
                    format!("{} has no file `{id}`", m.id),
                )
            }
            Err(e) => return internal(e),
        };
        let path = s.files.path(&m.id, &kept.id, thumb);
        match std::fs::read(&path) {
            Ok(bytes) => {
                let kind = if thumb {
                    "image/webp"
                } else {
                    kept.kind.content_type()
                };
                let mut res = Body::from(bytes).into_response();
                let h = res.headers_mut();
                h.insert(header::CONTENT_TYPE, HeaderValue::from_static(kind));
                h.insert(
                    header::CACHE_CONTROL,
                    HeaderValue::from_static("private, max-age=31536000, immutable"),
                );
                h.insert(
                    "x-content-type-options",
                    HeaderValue::from_static("nosniff"),
                );
                res
            }
            Err(e) => internal(format!("{}: {e}", path.display())),
        }
    })
    .await
    .unwrap_or_else(internal)
}

/// `GET /api/v1/files/{app}/{id}`.
pub async fn file(
    State(s): State<Arc<App>>,
    Path((app, id)): Path<(String, String)>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    serve_file(s, app, id, false, method, headers).await
}

/// `GET /api/v1/files/{app}/{id}/thumb`.
pub async fn thumb(
    State(s): State<Arc<App>>,
    Path((app, id)): Path<(String, String)>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    serve_file(s, app, id, true, method, headers).await
}
