//! `GET /v1/status` (§20.2): ready, the build, when it started, and the
//! store's schema — what the gateway reports as the site's health.

use std::sync::Arc;

use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

use crate::app::App;

pub async fn status(State(app): State<Arc<App>>) -> Response {
    let schema = app.store.version().ok();
    Json(json!({
        "state": if schema.is_some() { "ready" } else { "store_unreadable" },
        "build": app.build.label(),
        "started_at_ms": app.started_ms,
        "store": {"schema": schema},
    }))
    .into_response()
}
