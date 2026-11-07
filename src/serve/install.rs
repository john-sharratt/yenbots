//! What makes the site an installable web app (§11.1): its manifest at
//! `/_/manifest.webmanifest`, its icon — a library tile, as PNG for the
//! iPhone's home screen — and its service worker at `/sw.js`, which shows
//! each push and opens what a tap names. The head every page has links them
//! ([`HEAD`]).

use std::sync::Arc;

use axum::extract::State;
use axum::http::{header, HeaderMap, HeaderValue};
use axum::response::{IntoResponse, Response};
use serde_json::json;

use super::asset::respond;
use super::web::serve;
use crate::app::App;

/// The library tile that is the site's icon.
pub const ICON: &str = "tiles/heart-coral";

/// The site's background and its bar's colour on a phone: the palette's
/// paper and coral (`web/_/ui/tokens.css`).
const BACKGROUND: &str = "#fbf7f2";
const THEME: &str = "#d4573d";

/// The lines every page's head carries for it: the manifest, the icon an
/// iPhone puts on its home screen, the bar's colour, and leave to run
/// full-screen from there.
pub const HEAD: &[&str] = &[
    "<link rel=\"manifest\" href=\"/_/manifest.webmanifest\">",
    "<link rel=\"apple-touch-icon\" href=\"/_library/tiles/heart-coral-256.png\">",
    "<meta name=\"theme-color\" content=\"#d4573d\">",
    "<meta name=\"apple-mobile-web-app-capable\" content=\"yes\">",
];

/// `/_/manifest.webmanifest`.
pub async fn manifest(State(s): State<Arc<App>>, headers: HeaderMap) -> Response {
    let icon = |w: u32, kind: &str| {
        json!({
            "src": format!("/_library/{ICON}-{w}.{kind}"),
            "sizes": format!("{w}x{w}"),
            "type": format!("image/{kind}"),
            "purpose": "any",
        })
    };
    let body = json!({
        "name": s.cfg.site_name,
        "short_name": s.cfg.site_name,
        "start_url": "/",
        "scope": "/",
        "display": "standalone",
        "background_color": BACKGROUND,
        "theme_color": THEME,
        "icons": [icon(512, "png"), icon(256, "png"), icon(512, "webp")],
    })
    .to_string();
    respond("manifest.webmanifest", body.as_bytes(), &headers)
}

/// `/sw.js`, at the site's root so it may handle every page's pushes.
pub async fn service_worker(headers: HeaderMap) -> Response {
    let mut res = serve("sw.js", &headers);
    res.headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    res.into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The head names the icon the manifest does, and the bar is coral**;
    /// both are the palette's.
    #[test]
    fn the_head_and_the_manifest_agree() {
        assert!(HEAD[1].contains(&format!("/_library/{ICON}-256.png")));
        assert!(HEAD[2].contains(THEME));
        let tokens = include_str!("../../web/_/ui/tokens.css");
        assert!(tokens.contains(BACKGROUND), "the paper token");
        assert!(tokens.contains(THEME), "the coral token");
        let landing = include_str!("../../web/index.html");
        for line in HEAD {
            assert!(landing.contains(line), "the landing page's head has {line}");
        }
    }
}
