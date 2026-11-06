//! `/_/theme/<id>.css` (§8.5): an app's theme, written from its manifest as
//! the stylesheet that follows the platform's on every one of its pages. An
//! app with no theme answers an empty one. A page names it with the digest
//! of its contents (`?v=…`), so an edit to the theme is seen on the next
//! load, not a minute later.

use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};

use super::asset::{etag, respond};
use super::reply::internal;
use crate::app::App;
use crate::apps::theme::tokens::Tokens;
use crate::apps::theme::Theme;

/// The stylesheet for `theme`.
pub fn css(theme: &Theme) -> anyhow::Result<String> {
    theme.css(Tokens::platform())
}

/// The address a page links the theme of app `id` at: the stylesheet's
/// path, versioned by its contents.
pub fn href(id: &str, theme: &Theme) -> anyhow::Result<String> {
    let tag = etag(css(theme)?.as_bytes());
    Ok(format!("/_/theme/{id}.css?v={}", &tag[1..9]))
}

/// The answer to `/_/theme/<file>`, `file` being `<id>.css`.
pub fn stylesheet(s: &App, file: &str, headers: &HeaderMap) -> Response {
    let Some(manifest) = file.strip_suffix(".css").and_then(|id| s.apps.get(id)) else {
        return (StatusCode::NOT_FOUND, "no such app").into_response();
    };
    match css(&manifest.theme) {
        Ok(text) => respond("theme.css", text.as_bytes(), headers),
        Err(e) => internal(e),
    }
}
