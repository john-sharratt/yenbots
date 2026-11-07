//! The one responder every file goes through (§5.2), the platform's and the
//! apps' alike: a strong `ETag` over the content, `304` when the browser
//! already holds it, gzip for a browser that takes it, and a short `max-age`
//! for HTML, JS and CSS so an edit is picked up on the next load.

use std::io::Write;

use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use flate2::write::GzEncoder;
use flate2::Compression;
use sha2::{Digest, Sha256};

/// The content type a file is served with, by its extension.
pub fn content_type(path: &str) -> &'static str {
    match path.rsplit_once('.').map(|(_, ext)| ext) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") | Some("mjs") => "text/javascript; charset=utf-8",
        Some("json") => "application/json",
        Some("webmanifest") => "application/manifest+json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("woff2") => "font/woff2",
        Some("ico") => "image/x-icon",
        Some("txt") | Some("md") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// How long a browser may keep a file without asking: a minute for what an
/// edit changes, a day for pictures and type.
fn max_age(content_type: &str) -> &'static str {
    if content_type.starts_with("text/") || content_type == "application/json" {
        "max-age=60"
    } else {
        "max-age=86400"
    }
}

/// Whether a file of this type is worth compressing.
fn compressible(content_type: &str) -> bool {
    content_type.starts_with("text/")
        || content_type == "application/json"
        || content_type == "image/svg+xml"
}

/// The strong validator of `bytes`.
pub fn etag(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let hex: String = digest[..16].iter().map(|b| format!("{b:02x}")).collect();
    format!("\"{hex}\"")
}

/// Whether an `If-None-Match` names `tag`.
fn matches(headers: &HeaderMap, tag: &str) -> bool {
    headers
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.split(',').map(str::trim).any(|t| t == tag || t == "*"))
}

fn accepts_gzip(headers: &HeaderMap) -> bool {
    headers
        .get(header::ACCEPT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| {
            v.split(',')
                .map(|e| e.split(';').next().unwrap_or("").trim())
                .any(|e| e == "gzip")
        })
}

fn gzip(bytes: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut enc = GzEncoder::new(Vec::new(), Compression::default());
    enc.write_all(bytes)?;
    enc.finish()
}

/// Below this, gzip's header costs more than it saves.
const GZIP_FROM: usize = 512;

/// `bytes`, the file at `path`, answered to a request with `headers`.
pub fn respond(path: &str, bytes: &[u8], headers: &HeaderMap) -> Response {
    let ct = content_type(path);
    let tag = etag(bytes);
    let common = [
        (header::ETAG, tag.clone()),
        (header::CACHE_CONTROL, max_age(ct).to_string()),
        (header::VARY, "accept-encoding".to_string()),
    ];
    if matches(headers, &tag) {
        return (StatusCode::NOT_MODIFIED, common).into_response();
    }
    let zipped = (compressible(ct) && bytes.len() >= GZIP_FROM && accepts_gzip(headers))
        .then(|| gzip(bytes).ok())
        .flatten();
    let mut res = match zipped {
        Some(z) => {
            let mut r = (common, z).into_response();
            r.headers_mut()
                .insert(header::CONTENT_ENCODING, HeaderValue::from_static("gzip"));
            r
        }
        None => (common, bytes.to_vec()).into_response(),
    };
    res.headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(ct));
    res
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use axum::body::to_bytes;
    use flate2::read::GzDecoder;

    use super::*;

    #[test]
    fn files_are_typed_by_their_extension() {
        assert_eq!(content_type("index.html"), "text/html; charset=utf-8");
        assert_eq!(content_type("_/yen.js"), "text/javascript; charset=utf-8");
        assert_eq!(
            content_type("vendor/signals-core.mjs"),
            "text/javascript; charset=utf-8"
        );
        assert_eq!(content_type("_/ui/yen.css"), "text/css; charset=utf-8");
        assert_eq!(content_type("README"), "application/octet-stream");
    }

    #[test]
    fn a_tag_is_the_contents_digest() {
        assert_eq!(etag(b""), "\"e3b0c44298fc1c149afbf4c8996fb924\"");
    }

    fn asking(pairs: &[(header::HeaderName, &str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.insert(k, HeaderValue::from_str(v).unwrap());
        }
        h
    }

    /// **A file the browser holds is answered 304, and a large text file
    /// gzipped for a browser that takes it.**
    #[tokio::test]
    async fn a_file_is_validated_and_compressed() {
        let text = "const x = 1;\n".repeat(100);
        let plain = respond("app.js", text.as_bytes(), &HeaderMap::new());
        assert_eq!(plain.status(), StatusCode::OK);
        assert_eq!(
            plain.headers()[header::CONTENT_TYPE],
            "text/javascript; charset=utf-8"
        );
        assert_eq!(plain.headers()[header::CACHE_CONTROL], "max-age=60");
        assert!(plain.headers().get(header::CONTENT_ENCODING).is_none());
        let tag = plain.headers()[header::ETAG].to_str().unwrap().to_string();

        let held = respond(
            "app.js",
            text.as_bytes(),
            &asking(&[(header::IF_NONE_MATCH, &tag)]),
        );
        assert_eq!(held.status(), StatusCode::NOT_MODIFIED);

        let gz = asking(&[(header::ACCEPT_ENCODING, "br;q=1.0, gzip;q=0.8")]);
        let zipped = respond("app.js", text.as_bytes(), &gz);
        assert_eq!(zipped.headers()[header::CONTENT_ENCODING], "gzip");
        assert_eq!(zipped.headers()[header::ETAG], tag.as_str());
        let body = to_bytes(zipped.into_body(), 1 << 20).await.unwrap();
        let mut back = String::new();
        GzDecoder::new(&body[..]).read_to_string(&mut back).unwrap();
        assert_eq!(back, text);

        let small = respond("a.css", b"p{}", &gz);
        assert!(small.headers().get(header::CONTENT_ENCODING).is_none());
        let picture = respond("t.png", &[0u8; 2048], &gz);
        assert!(picture.headers().get(header::CONTENT_ENCODING).is_none());
        assert_eq!(picture.headers()[header::CACHE_CONTROL], "max-age=86400");
    }
}
