//! The platform's web files, embedded at build time from `web/`: the landing
//! page at `/`, and everything under `web/_/` at `/_/…`.

use axum::extract::Path;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use include_dir::{include_dir, Dir};

static WEB: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/web");

/// The content type a file is served with, by its extension.
pub fn content_type(path: &str) -> &'static str {
    match path.rsplit_once('.').map(|(_, ext)| ext) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") | Some("mjs") => "text/javascript; charset=utf-8",
        Some("json") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        Some("woff2") => "font/woff2",
        Some("ico") => "image/x-icon",
        _ => "application/octet-stream",
    }
}

/// The embedded file at `path`, relative to `web/`.
fn serve(path: &str) -> Response {
    match WEB.get_file(path) {
        Some(file) => (
            [
                (header::CONTENT_TYPE, content_type(path)),
                (header::CACHE_CONTROL, "max-age=60"),
            ],
            file.contents(),
        )
            .into_response(),
        None => (StatusCode::NOT_FOUND, "no such file").into_response(),
    }
}

pub async fn landing() -> Response {
    serve("index.html")
}

pub async fn platform_file(Path(path): Path<String>) -> Response {
    // `include_dir` names files by their path under `web/`, so nothing
    // outside it can be named; a `..` is refused before the lookup anyway.
    if path.split('/').any(|seg| seg == ".." || seg.is_empty()) {
        return (StatusCode::NOT_FOUND, "no such file").into_response();
    }
    serve(&format!("_/{path}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn files_are_typed_by_their_extension() {
        assert_eq!(content_type("index.html"), "text/html; charset=utf-8");
        assert_eq!(
            content_type("_/framework/yen.js"),
            "text/javascript; charset=utf-8"
        );
        assert_eq!(content_type("_/ui/yen.css"), "text/css; charset=utf-8");
        assert_eq!(content_type("README"), "application/octet-stream");
    }

    #[test]
    fn the_landing_page_is_built_in() {
        assert!(WEB.get_file("index.html").is_some());
    }
}
