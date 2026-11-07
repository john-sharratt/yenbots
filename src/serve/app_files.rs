//! `/a/{id}/…` (§5.2, §8.3): an app's files, read from its folder in the
//! live checkout on every request, so an edit is live on the next load.
//!
//! - A path with no file and no extension is one of the app's pages — a
//!   deep link such as `/a/recipes/r/7` — and is answered with the app's
//!   `index.html`; the framework's router reads the page from the address.
//! - `index.html` is given `<base href="/a/<id>/">`, so its relative
//!   addresses — `app.js`, a link to `r/7` — mean the same from every page;
//!   and the platform's look, `/_/yen.css`, then the app's theme (§8.5),
//!   before anything of the app's own.
//! - The manifest and the workflows are the server's, and are not served.
//! - An app whose files in the working tree the check refused is served
//!   from the commit it last passed at ([`FilesAt`]).

use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};

use super::asset::respond;
use super::install::HEAD;
use super::reply::internal;
use super::theme::href;
use crate::app::App;
use crate::apps::FilesAt;
use crate::git::git_bytes;
use crate::lifecycle::rename::now_named;

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

/// `html` with the head every app page has as the first thing in its head:
/// `<base href="/a/<id>/">` (unless it names a base already), the platform's
/// stylesheet, the app's theme at `theme`, and the lines that make the site
/// installable ([`HEAD`]).
pub fn with_head(html: &str, id: &str, theme: &str) -> String {
    let lower = html.to_ascii_lowercase();
    let mut lines = Vec::new();
    if !lower.contains("<base") {
        lines.push(format!("<base href=\"/a/{id}/\">"));
    }
    lines.push("<link rel=\"stylesheet\" href=\"/_/yen.css\">".to_string());
    lines.push(format!("<link rel=\"stylesheet\" href=\"{theme}\">"));
    lines.extend(HEAD.iter().map(|l| l.to_string()));
    let head = lower.match_indices("<head").map(|(at, _)| at).find(|&at| {
        matches!(
            lower.as_bytes().get(at + 5),
            Some(b'>' | b' ' | b'\t' | b'\n' | b'\r')
        )
    });
    match head.and_then(|at| lower[at..].find('>').map(|end| at + end + 1)) {
        Some(after) => {
            let added: String = lines.iter().map(|l| format!("\n  {l}")).collect();
            format!("{}{added}{}", &html[..after], &html[after..])
        }
        None => format!("{}\n{html}", lines.join("\n")),
    }
}

fn page(id: &str, theme: &str, bytes: Vec<u8>, headers: &HeaderMap) -> Response {
    let html = with_head(&String::from_utf8_lossy(&bytes), id, theme);
    respond("index.html", html.as_bytes(), headers)
}

/// Where app `id`'s files are read from: its folder in the working tree, or
/// the commit it last passed the check at.
struct Source {
    root: PathBuf,
    id: String,
    at: FilesAt,
}

impl Source {
    fn read(&self, rel: &str) -> Option<Vec<u8>> {
        match &self.at {
            FilesAt::Tree => std::fs::read(self.root.join("apps").join(&self.id).join(rel)).ok(),
            FilesAt::Commit(commit) => git_bytes(
                &self.root,
                &["show", &format!("{commit}:apps/{}/{rel}", self.id)],
            )
            .ok(),
        }
    }
}

/// The file `rel` of the app, or its page, with its theme at `theme`.
fn read(src: &Source, theme: &str, rel: &str, headers: &HeaderMap) -> Response {
    let rel = if rel.is_empty() { "index.html" } else { rel };
    match src.read(rel) {
        Some(bytes) if rel == "index.html" => page(&src.id, theme, bytes, headers),
        Some(bytes) => respond(rel, &bytes, headers),
        None if is_page(rel) => match src.read("index.html") {
            Some(bytes) => page(&src.id, theme, bytes, headers),
            None => not_found(),
        },
        None => not_found(),
    }
}

async fn serve(s: Arc<App>, id: String, rel: String, headers: HeaderMap) -> Response {
    let (Some(manifest), Some(at)) = (s.apps.get(&id), s.apps.files_at(&id)) else {
        // An app renamed is reached at its old address too (§20.1).
        return match now_named(&s.store.conn(), &id) {
            Ok(Some(new)) => Redirect::permanent(&format!("/a/{new}/{rel}")).into_response(),
            Ok(None) => not_found(),
            Err(e) => internal(e),
        };
    };
    if !(rel.is_empty() || served(&rel)) {
        return not_found();
    }
    let theme = match href(&id, &manifest.theme) {
        Ok(theme) => theme,
        Err(e) => return internal(e),
    };
    let src = Source {
        root: s.apps.root().to_path_buf(),
        id,
        at,
    };
    tokio::task::spawn_blocking(move || read(&src, &theme, &rel, &headers))
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

    const LOOK: &str = "<link rel=\"stylesheet\" href=\"/_/yen.css\">";

    #[test]
    fn the_base_and_the_look_are_the_first_things_in_the_head() {
        // The install lines, indented in a head and not outside one.
        let inside: String = HEAD.iter().map(|l| format!("\n  {l}")).collect();
        let outside = HEAD.join("\n");
        let theme = "/_/theme/recipes.css?v=1";
        let html = "<!doctype html>\n<html>\n<HEAD lang=\"en\">\n  <title>R</title>\n</head>";
        assert_eq!(
            with_head(html, "recipes", theme),
            format!(
                "<!doctype html>\n<html>\n<HEAD lang=\"en\">\n  <base href=\"/a/recipes/\">\n  {LOOK}\n  \
                 <link rel=\"stylesheet\" href=\"/_/theme/recipes.css?v=1\">{inside}\n  <title>R</title>\n</head>"
            )
        );
        assert_eq!(
            with_head("<head><base href=\"/x/\"></head>", "recipes", theme),
            format!(
                "<head>\n  {LOOK}\n  <link rel=\"stylesheet\" href=\"/_/theme/recipes.css?v=1\">{inside}<base \
                 href=\"/x/\"></head>"
            ),
            "a base of its own is kept"
        );
        assert_eq!(
            with_head("<yen-app></yen-app>", "r", "/t.css"),
            format!(
                "<base href=\"/a/r/\">\n{LOOK}\n<link rel=\"stylesheet\" href=\"/t.css\">\n{outside}\n<yen-app></yen-app>"
            )
        );
        // `<header>` is not the head.
        assert_eq!(
            with_head("<header></header>", "r", "/t.css"),
            format!(
                "<base href=\"/a/r/\">\n{LOOK}\n<link rel=\"stylesheet\" href=\"/t.css\">\n{outside}\n<header></header>"
            )
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
