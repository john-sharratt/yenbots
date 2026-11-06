//! `GET /api/v1/me`: who the caller is. A signed-in person's first call
//! registers their account (§12.2); a pending account learns it is waiting;
//! an API token answers the account it belongs to.

use std::sync::Arc;

use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::Response;
use serde_json::json;

use super::reply::{error, internal, ok};
use crate::accounts::caller::{who, Who};
use crate::app::App;

pub async fn me(State(app): State<Arc<App>>, headers: HeaderMap) -> Response {
    let token = headers.contains_key(header::AUTHORIZATION);
    let a = app.clone();
    let found = match tokio::task::spawn_blocking(move || who(&a.store, &a.cfg, &headers)).await {
        Ok(Ok(w)) => w,
        Ok(Err(e)) => return internal(e),
        Err(e) => return internal(e),
    };
    let site = &app.cfg.site_name;
    match found {
        Who::Nobody if token => error(
            StatusCode::UNAUTHORIZED,
            "bad_token",
            "the token is not one, or has expired",
        ),
        Who::Nobody => ok(json!({"signed_in": false, "site": site})),
        Who::Waiting(account) => ok(json!({"signed_in": true, "site": site, "account": account})),
        Who::Active(c) => ok(json!({
            "signed_in": true,
            "site": site,
            "account": c.account,
            "groups": c.groups,
            "token": c.token,
        })),
    }
}
