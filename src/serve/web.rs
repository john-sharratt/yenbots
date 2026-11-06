//! The platform's web files, embedded at build time from `web/`: the landing
//! page at `/`, and everything under `web/_/` at `/_/…` — the framework
//! (`/_/yen.js`), its vendored modules and the site's own files.

use axum::extract::Path;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use include_dir::{include_dir, Dir};

use super::asset::respond;

static WEB: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/web");

/// The embedded file at `path`, relative to `web/`.
fn serve(path: &str, headers: &HeaderMap) -> Response {
    match WEB.get_file(path) {
        Some(file) => respond(path, file.contents(), headers),
        None => (StatusCode::NOT_FOUND, "no such file").into_response(),
    }
}

pub async fn landing(headers: HeaderMap) -> Response {
    serve("index.html", &headers)
}

pub async fn platform_file(Path(path): Path<String>, headers: HeaderMap) -> Response {
    // `include_dir` names files by their path under `web/`, so nothing
    // outside it can be named; a `..` is refused before the lookup anyway.
    if path.split('/').any(|seg| seg == ".." || seg.is_empty()) {
        return (StatusCode::NOT_FOUND, "no such file").into_response();
    }
    serve(&format!("_/{path}"), &headers)
}

#[cfg(test)]
mod tests {
    use sha2::{Digest, Sha256};

    use super::*;

    #[test]
    fn the_landing_page_and_the_framework_are_built_in() {
        for path in ["index.html", "_/yen.js", "_/framework/element.js"] {
            assert!(WEB.get_file(path).is_some(), "{path}");
        }
    }

    fn digest(path: &str) -> String {
        let bytes = WEB.get_file(path).unwrap().contents();
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    /// **The vendored modules are the pinned releases, byte for byte**:
    /// `lit-core.min.js` is `lit/dist@3.2.1/core/lit-core.min.js` (Lit 3.2.1),
    /// `signals-core.mjs` is `@preact/signals-core@1.8.0/dist/signals-core.mjs`.
    #[test]
    fn the_vendored_modules_are_the_pinned_releases() {
        assert_eq!(
            digest("_/vendor/lit-core.min.js"),
            "65a540523226c59100a9f8c72c7c0af4d85c79fb80cabc14a1186af007428645"
        );
        assert_eq!(
            digest("_/vendor/signals-core.mjs"),
            "93284268a0e37da474d3a3fa80cfd7473ba0c00827b8ffd4aab61894bd74d69f"
        );
    }
}
