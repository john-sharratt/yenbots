//! `/a/{id}/…` (§5.2, §8.3): an app's files, read from its folder in the
//! live checkout on every request, so an edit is live on the next load.
//!
//! - A path with no file and no extension is one of the app's pages — a
//!   deep link such as `/a/recipes/r/7` — and is answered with the app's
//!   `index.html`; the framework's router reads the page from the address.
//! - `index.html` is given `<base href="/a/<id>/">`, so its relative
//!   addresses — `app.js`, a link to `r/7` — mean the same from every page.
//! - The manifest and the workflows are the server's, and are not served.

use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};

use super::asset::respond;
use super::reply::internal;
use crate::app::App;

fn not_found() -> Response {
    (StatusCode::NOT_FOUND, "no such file").into_response()
}

/// Whether `rel` names a file an app serves: inside its folder — no `..`,
/// no hidden file, and none of the separators or drive marks Windows would
/// read as a way out — and not the server's own.
fn served(rel: &str) -> bool {
    let mut segments = rel.split('/');
    let first = segments.clone().next().unwrap_or("");
    rel != "app.yaml"
        && first != "workflows"
        && segments.all(|s| !s.is_empty() && !s.starts_with('.') && !s.contains(['\\', ':']))
}

/// Whether `rel` names one of the app's pages rather than a file.
fn is_page(rel: &str) -> bool {
    !rel.rsplit('/').next().unwrap_or("").contains('.')
}

/// `html` with `<base href="/a/<id>/">` as the first thing in its head,
/// unless it names a base already.
pub fn with_base(html: &str, id: &str) -> String {
    let lower = html.to_ascii_lowercase();
    if lower.contains("<base") {
        return html.to_string();
    }
    let base = format!("<base href=\"/a/{id}/\">");
    let head = lower.match_indices("<head").map(|(at, _)| at).find(|&at| {
        matches!(
            lower.as_bytes().get(at + 5),
            Some(b'>' | b' ' | b'\t' | b'\n' | b'\r')
        )
    });
    match head.and_then(|at| lower[at..].find('>').map(|end| at + end + 1)) {
        Some(after) => format!("{}\n  {base}{}", &html[..after], &html[after..]),
        None => format!("{base}\n{html}"),
    }
}

fn page(id: &str, bytes: Vec<u8>, headers: &HeaderMap) -> Response {
    let html = with_base(&String::from_utf8_lossy(&bytes), id);
    respond("index.html", html.as_bytes(), headers)
}

/// The file `rel` of app `id`, or its page.
fn read(dir: PathBuf, id: &str, rel: &str, headers: &HeaderMap) -> Response {
    let rel = if rel.is_empty() { "index.html" } else { rel };
    match std::fs::read(dir.join(rel)) {
        Ok(bytes) if rel == "index.html" => page(id, bytes, headers),
        Ok(bytes) => respond(rel, &bytes, headers),
        Err(_) if is_page(rel) => match std::fs::read(dir.join("index.html")) {
            Ok(bytes) => page(id, bytes, headers),
            Err(_) => not_found(),
        },
        Err(_) => not_found(),
    }
}

async fn serve(s: Arc<App>, id: String, rel: String, headers: HeaderMap) -> Response {
    if s.apps.get(&id).is_none() || !(rel.is_empty() || served(&rel)) {
        return not_found();
    }
    let dir = s.apps.dir().join(&id);
    tokio::task::spawn_blocking(move || read(dir, &id, &rel, &headers))
        .await
        .unwrap_or_else(internal)
}

/// `/a/{id}`: the app is its folder, so its address ends in a slash.
pub async fn bare(Path(id): Path<String>) -> Redirect {
    Redirect::permanent(&format!("/a/{id}/"))
}

pub async fn index(
    State(s): State<Arc<App>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Response {
    serve(s, id, String::new(), headers).await
}

pub async fn file(
    State(s): State<Arc<App>>,
    Path((id, rel)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    serve(s, id, rel, headers).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_base_is_the_first_thing_in_the_head() {
        let html = "<!doctype html>\n<html>\n<HEAD lang=\"en\">\n  <title>R</title>\n</head>";
        assert_eq!(
            with_base(html, "recipes"),
            "<!doctype html>\n<html>\n<HEAD lang=\"en\">\n  <base href=\"/a/recipes/\">\n  \
             <title>R</title>\n</head>"
        );
        let own = "<head><base href=\"/x/\"></head>";
        assert_eq!(with_base(own, "recipes"), own);
        assert_eq!(
            with_base("<yen-app></yen-app>", "r"),
            "<base href=\"/a/r/\">\n<yen-app></yen-app>"
        );
        // `<header>` is not the head.
        assert_eq!(
            with_base("<header></header>", "r"),
            "<base href=\"/a/r/\">\n<header></header>"
        );
    }

    #[test]
    fn the_servers_files_are_not_served() {
        assert!(served("app.js"));
        assert!(served("pages/home.js"));
        assert!(!served("app.yaml"));
        assert!(!served("workflows/remind.js"));
        assert!(!served("../other/app.js"));
        assert!(!served(".git/config"));
        assert!(!served("pages//home.js"));
        assert!(!served("pages\\..\\..\\secret.txt"));
        assert!(!served("C:/Windows/win.ini"));
        assert!(is_page("r/7"));
        assert!(!is_page("pages/home.js"));
    }
}
