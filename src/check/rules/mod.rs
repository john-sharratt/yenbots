//! The check's rules (§17.4), one per file. Each reads an app's [`Files`]
//! and pushes a [`Refusal`] for each thing it refuses.
//!
//! [`Files`]: super::files::Files
//! [`Refusal`]: super::Refusal

pub mod guide;
pub mod layout;
pub mod manifest;
pub mod markup;
pub mod outside;
pub mod references;
pub mod tokens;
pub mod workflows;

use regex::Regex;

use super::files::Files;
use super::source::{css_without_comments, tagged, without_comments, Template};

/// `apps/<id>/`.
pub fn app_dir(id: &str) -> String {
    format!("apps/{id}/")
}

/// The JavaScript files of app `id`, each with its comments blanked.
pub fn scripts(files: &Files, id: &str) -> Vec<(String, String)> {
    let dir = app_dir(id);
    files
        .texts_under(&dir)
        .filter(|(p, _)| p.ends_with(".js") || p.ends_with(".mjs"))
        .map(|(p, t)| (p.to_string(), without_comments(t)))
        .collect()
}

/// A piece of an app's markup or styles: the file, the text, and where in
/// the file the text starts.
pub struct Piece {
    pub path: String,
    /// The whole file, to count lines in.
    pub file: String,
    pub start: usize,
    pub text: String,
    /// The piece as written, `${…}` included, for a template; the text
    /// otherwise.
    pub raw: String,
}

/// `index.html`'s text, as a piece, when the app has one.
fn index_html(files: &Files, id: &str) -> Option<Piece> {
    let path = format!("{}index.html", app_dir(id));
    let file = files.text.get(&path)?.clone();
    let text = html_without_comments(&file);
    Some(Piece {
        path,
        raw: text.clone(),
        text,
        file,
        start: 0,
    })
}

/// `html` with its `<!-- … -->` comments blanked.
fn html_without_comments(html: &str) -> String {
    let mut out = html.as_bytes().to_vec();
    let mut i = 0;
    while let Some(n) = html[i..].find("<!--") {
        let at = i + n;
        let end = html[at..].find("-->").map_or(html.len(), |e| at + e + 3);
        for b in &mut out[at..end] {
            if *b != b'\n' {
                *b = b' ';
            }
        }
        i = end;
    }
    String::from_utf8(out).unwrap_or_else(|_| html.to_string())
}

fn pieces_of(path: &str, js: &str, tag: &str) -> Vec<Piece> {
    tagged(js, tag)
        .into_iter()
        .map(|Template { start, raw, text }| Piece {
            path: path.to_string(),
            file: js.to_string(),
            start,
            text,
            raw,
        })
        .collect()
}

/// The app's markup: every `html` and `svg` template and `index.html`.
pub fn markup(files: &Files, id: &str) -> Vec<Piece> {
    let mut out = Vec::new();
    for (path, js) in scripts(files, id) {
        out.extend(pieces_of(&path, &js, "html"));
        out.extend(pieces_of(&path, &js, "svg"));
    }
    out.extend(index_html(files, id));
    out
}

/// The app's styles: `.css` files, every `css` template, and each
/// `<style>` in `index.html` — comments blanked.
pub fn styles(files: &Files, id: &str) -> Vec<Piece> {
    let mut out = Vec::new();
    for (path, text) in files.texts_under(&app_dir(id)) {
        if path.ends_with(".css") {
            let clean = css_without_comments(text);
            out.push(Piece {
                path: path.to_string(),
                file: text.to_string(),
                start: 0,
                raw: clean.clone(),
                text: clean,
            });
        }
    }
    for (path, js) in scripts(files, id) {
        for mut p in pieces_of(&path, &js, "css") {
            p.text = css_without_comments(&p.text);
            out.push(p);
        }
    }
    if let Some(index) = index_html(files, id) {
        let style = Regex::new(r"(?is)<style\b[^>]*>(.*?)</style>").unwrap();
        for m in style.captures_iter(&index.text) {
            let body = m.get(1).unwrap();
            let clean = css_without_comments(body.as_str());
            out.push(Piece {
                path: index.path.clone(),
                file: index.file.clone(),
                start: body.start(),
                raw: clean.clone(),
                text: clean,
            });
        }
    }
    out
}

/// Whether `url` names something outside the site.
pub fn is_outside(url: &str) -> bool {
    let u = url.trim().to_ascii_lowercase();
    ["http:", "https:", "ws:", "wss:", "//"]
        .iter()
        .any(|p| u.starts_with(p))
}
