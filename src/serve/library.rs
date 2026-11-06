//! The art library on the site (§15.3): each picture at its sizes —
//! `/_library/<kind>/<id>-<width>.webp` — and the search the library page
//! and an app's `app.library.find` call, `GET /api/v1/library`. Neither
//! needs an account: the pictures are the site's own, like its framework.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::json;

use super::asset::respond;
use super::reply::{error, internal, ok, Args};
use crate::app::App;
use crate::library::dominant::PALETTE;
use crate::library::kind::Kind;
use crate::library::Find;

pub async fn picture(
    State(s): State<Arc<App>>,
    Path((kind, file)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    let Some(path) = s.library.file(&kind, &file) else {
        return (StatusCode::NOT_FOUND, "no such picture").into_response();
    };
    match tokio::fs::read(&path).await {
        Ok(bytes) => respond(&file, &bytes, &headers),
        Err(e) => internal(format!("{}: {e}", path.display())),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Asked {
    kind: Option<String>,
    tag: Option<String>,
    colour: Option<String>,
}

pub async fn find(State(s): State<Arc<App>>, Args(q): Args<Asked>) -> Response {
    let kind = match q.kind.as_deref().map(|k| (k, Kind::parse(k))) {
        None => None,
        Some((_, Some(k))) => Some(k),
        Some((k, None)) => {
            return error(
                StatusCode::BAD_REQUEST,
                "bad_kind",
                format!(
                    "`{k}` is not a kind: tiles, illustrations, empty, backgrounds or stickers"
                ),
            )
        }
    };
    if let Some(c) = q.colour.as_deref().filter(|c| !PALETTE.contains(c)) {
        return error(
            StatusCode::BAD_REQUEST,
            "bad_colour",
            format!("`{c}` is not a palette colour: {}", PALETTE.join(", ")),
        );
    }
    let found = s.library.find(&Find {
        kind,
        tag: q.tag,
        colour: q.colour,
    });
    let shown: Vec<_> = found.iter().map(|p| p.shown()).collect();
    ok(json!({ "pictures": shown }))
}
