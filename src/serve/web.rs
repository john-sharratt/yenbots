//! The platform's web files, embedded at build time from `web/`: the landing
//! page at `/`, everything under `web/_/` at `/_/…` — the framework
//! (`/_/yen.js`), the design system (`/_/yen.css`, `/_/ui/`), its icons and
//! faces, the vendored modules and the site's own files — an app's theme at
//! `/_/theme/<id>.css` ([`theme`](super::theme)), and the design system's
//! catalogue at `/_design` with its Vietnamese test page at
//! `/_design/vietnamese` (§8.5, §8.7).

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use include_dir::{include_dir, Dir};

use super::asset::respond;
use super::theme::stylesheet;
use crate::app::App;

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

/// Whether `path` could name a file under a folder: no `..`, no empty segment.
fn plain(path: &str) -> bool {
    !path.split('/').any(|seg| seg == ".." || seg.is_empty())
}

pub async fn platform_file(
    State(s): State<Arc<App>>,
    Path(path): Path<String>,
    headers: HeaderMap,
) -> Response {
    // `include_dir` names files by their path under `web/`, so nothing
    // outside it can be named; a `..` is refused before the lookup anyway.
    if !plain(&path) {
        return (StatusCode::NOT_FOUND, "no such file").into_response();
    }
    if let Some(file) = path.strip_prefix("theme/") {
        return stylesheet(&s, file, &headers);
    }
    serve(&format!("_/{path}"), &headers)
}

/// `/_design`: the catalogue of every token and component (§8.5).
pub async fn design(headers: HeaderMap) -> Response {
    serve("_design/index.html", &headers)
}

/// `/_design/…`: the catalogue's own files, and its pages by name —
/// `/_design/vietnamese` is `vietnamese.html`.
pub async fn design_file(Path(path): Path<String>, headers: HeaderMap) -> Response {
    if !plain(&path) {
        return (StatusCode::NOT_FOUND, "no such file").into_response();
    }
    let named = if path.rsplit('/').next().unwrap_or("").contains('.') {
        path
    } else {
        format!("{path}.html")
    };
    serve(&format!("_design/{named}"), &headers)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use include_dir::File;
    use regex::Regex;
    use sha2::{Digest, Sha256};
    use unicode_normalization::UnicodeNormalization;

    use super::*;

    #[test]
    fn the_landing_page_and_the_framework_are_built_in() {
        for path in [
            "index.html",
            "_/yen.js",
            "_/yen.css",
            "_/framework/element.js",
            "_/ui/tokens.css",
            "_/ui/components/index.js",
            "_/icons/sprite.svg",
            "_design/index.html",
            "_design/vietnamese.html",
        ] {
            assert!(WEB.get_file(path).is_some(), "{path}");
        }
    }

    fn text(path: &str) -> &'static str {
        WEB.get_file(path).unwrap().contents_utf8().unwrap()
    }

    /// Every file under `dir`, at any depth.
    fn every_file(dir: &'static Dir<'static>) -> Vec<&'static File<'static>> {
        let mut out: Vec<&File> = dir.files().collect();
        for sub in dir.dirs() {
            out.extend(every_file(sub));
        }
        out
    }

    /// **The icon names and the sprite agree**: every name `icons.js` lists
    /// is a symbol in the sprite, and every symbol is listed.
    #[test]
    fn the_icon_names_are_the_sprites() {
        let listed: Vec<&str> = text("_/ui/icons.js")
            .split("export const ICONS = [")
            .nth(1)
            .unwrap()
            .split(']')
            .next()
            .unwrap()
            .split(',')
            .map(|s| s.trim().trim_matches('"'))
            .filter(|s| !s.is_empty())
            .collect();
        let drawn: Vec<&str> = text("_/icons/sprite.svg")
            .split("<symbol id=\"")
            .skip(1)
            .map(|s| s.split('"').next().unwrap())
            .collect();
        assert_eq!(listed, drawn);
    }

    /// **Every icon the platform's own files name is drawn**: a component
    /// or page asking for an icon the sprite lacks would show a blank.
    #[test]
    fn every_icon_used_is_drawn() {
        let drawn: Vec<String> = text("_/icons/sprite.svg")
            .split("<symbol id=\"")
            .skip(1)
            .map(|s| s.split('"').next().unwrap().to_string())
            .collect();
        let used = Regex::new(r#"(?:<yen-icon[^>]*\bname|\bicon|empty-icon)="([a-z-]+)""#).unwrap();
        let mut seen = 0;
        for f in every_file(&WEB) {
            let path = f.path().to_string_lossy();
            if !(path.ends_with(".js") || path.ends_with(".html")) || path.contains("vendor") {
                continue;
            }
            for c in used.captures_iter(f.contents_utf8().unwrap_or("")) {
                seen += 1;
                assert!(
                    drawn.contains(&c[1].to_string()),
                    "{}: no icon `{}`",
                    f.path().display(),
                    &c[1]
                );
            }
        }
        assert!(seen > 20, "the icons used are found: {seen}");
    }

    /// The code points `unicode-range` names: `U+0102-0103, U+0323` is 0x102,
    /// 0x103 and 0x323.
    fn ranges(text: &str) -> Vec<(u32, u32)> {
        text.split(',')
            .map(|r| {
                let r = r.trim().trim_start_matches("U+");
                let (lo, hi) = r.split_once('-').unwrap_or((r, r));
                (
                    u32::from_str_radix(lo, 16).unwrap(),
                    u32::from_str_radix(hi, 16).unwrap(),
                )
            })
            .collect()
    }

    /// **Both faces take every Vietnamese letter (§8.7)**: at each weight,
    /// the ranges `fonts.css` declares for a face cover every letter — the
    /// twelve vowels with each tone, d and đ, small and capital, 148 in all —
    /// composed, so no letter falls through to another face.
    #[test]
    fn both_faces_cover_every_vietnamese_letter() {
        let mut faces: BTreeMap<(String, String), Vec<(u32, u32)>> = BTreeMap::new();
        for block in text("_/ui/fonts.css").split("@font-face {").skip(1) {
            let value = |key: &str| {
                block
                    .split(&format!("{key}:"))
                    .nth(1)
                    .unwrap()
                    .split(';')
                    .next()
                    .unwrap()
                    .trim()
                    .to_string()
            };
            let face = (value("font-family"), value("font-weight"));
            faces
                .entry(face)
                .or_default()
                .extend(ranges(&value("unicode-range")));
        }
        assert_eq!(
            faces.len(),
            5,
            "Be Vietnam Pro at four weights, Fraunces at one: {:?}",
            faces.keys()
        );
        let mut letters = Vec::new();
        for base in "aăâeêioôơuưyAĂÂEÊIOÔƠUƯY".chars() {
            for tone in ["", "\u{300}", "\u{301}", "\u{309}", "\u{303}", "\u{323}"] {
                let composed: String = format!("{base}{tone}").nfc().collect();
                assert_eq!(composed.chars().count(), 1, "{composed}");
                letters.push(composed.chars().next().unwrap());
            }
        }
        letters.extend(['d', 'D', 'đ', 'Đ']);
        assert_eq!(letters.len(), 148);
        for (face, covered) in &faces {
            for l in &letters {
                let cp = u32::from(*l);
                assert!(
                    covered.iter().any(|(lo, hi)| (*lo..=*hi).contains(&cp)),
                    "{face:?} lacks {l} U+{cp:04X}"
                );
            }
        }
    }

    /// **Every face the type names is embedded**: each `url(…)` in
    /// `fonts.css` is a file under `web/_/fonts/`.
    #[test]
    fn every_face_is_embedded() {
        let urls: Vec<&str> = text("_/ui/fonts.css")
            .split("url(\"../fonts/")
            .skip(1)
            .map(|s| s.split('"').next().unwrap())
            .collect();
        assert_eq!(urls.len(), 15);
        for u in urls {
            assert!(WEB.get_file(format!("_/fonts/{u}")).is_some(), "{u}");
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
    /// `lit-all.min.js` is `lit/dist@3.2.1/all/lit-all.min.js` (Lit 3.2.1 with
    /// its directives), `signals-core.mjs` is
    /// `@preact/signals-core@1.8.0/dist/signals-core.mjs`.
    #[test]
    fn the_vendored_modules_are_the_pinned_releases() {
        assert_eq!(
            digest("_/vendor/lit-all.min.js"),
            "f92bc47241abc95bfbd6f6687beed90751d4d6cb4adb60ae58094e7b32e7fa72"
        );
        assert_eq!(
            digest("_/vendor/signals-core.mjs"),
            "93284268a0e37da474d3a3fa80cfd7473ba0c00827b8ffd4aab61894bd74d69f"
        );
    }
}
