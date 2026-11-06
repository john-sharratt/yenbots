//! `GET /api/v1/me`: who the caller is. A signed-in person's first call
//! registers their account (§12.2); a pending account learns it is waiting.

use std::sync::Arc;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

use crate::accounts::identity::identify;
use crate::accounts::{now_ms, sign_in};
use crate::app::App;

pub async fn me(State(app): State<Arc<App>>, headers: HeaderMap) -> Response {
    let Some(id) = identify(&headers) else {
        return Json(json!({"signed_in": false, "site": app.cfg.site_name})).into_response();
    };
    let app2 = app.clone();
    let answer =
        tokio::task::spawn_blocking(move || sign_in(&app2.store, &app2.cfg, &id, now_ms())).await;
    match answer {
        Ok(Ok(account)) => Json(json!({
            "signed_in": true,
            "site": app.cfg.site_name,
            "account": account,
        }))
        .into_response(),
        Ok(Err(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "store", "detail": e.to_string()})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "panic", "detail": e.to_string()})),
        )
            .into_response(),
    }
}
