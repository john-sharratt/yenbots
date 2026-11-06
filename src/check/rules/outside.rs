//! `outside`: an app loads nothing from outside the site (§8.1, §12.6) — no
//! script, style, picture, font or call to another address. What an app
//! needs is the site's: the framework, the library, its own files, the API.
//! A link a person follows (`<a href="https://…">`) loads nothing, and is not
//! refused.

use regex::Regex;

use crate::check::files::Files;
use crate::check::source::line_of;
use crate::check::Refusal;

use super::{is_outside, markup, scripts, styles};

pub const RULE: &str = "outside";

fn refusal(path: &str, line: usize, url: &str) -> Refusal {
    Refusal::new(
        RULE,
        path,
        Some(line),
        format!("loads `{url}` from outside the site: everything an app loads is the site's own"),
    )
}

pub fn check(files: &Files, id: &str, out: &mut Vec<Refusal>) {
    let imports = Regex::new(r#"(?:\bfrom\s*|\bimport\s*\(?\s*)["'`]([^"'`]+)["'`]"#).unwrap();
    let calls = Regex::new(r#"\b(?:fetch|EventSource|WebSocket|Worker|SharedWorker|importScripts|sendBeacon)\s*\(\s*["'`]([^"'`]+)["'`]"#).unwrap();
    for (path, js) in scripts(files, id) {
        for re in [&imports, &calls] {
            for c in re.captures_iter(&js) {
                let url = c.get(1).unwrap();
                if is_outside(url.as_str()) {
                    out.push(refusal(&path, line_of(&js, url.start()), url.as_str()));
                }
            }
        }
    }

    let loads = Regex::new(
        r#"(?is)<(?:script|img|iframe|video|audio|source|track|embed|object|link|image|use|input)\b[^>]*?\s(?:src|href|srcset|data|poster|xlink:href)\s*=\s*["']?([^"'\s>]+)"#,
    )
    .unwrap();
    for piece in markup(files, id) {
        for c in loads.captures_iter(&piece.text) {
            let url = c.get(1).unwrap();
            if is_outside(url.as_str()) {
                out.push(refusal(
                    &piece.path,
                    line_of(&piece.file, piece.start + url.start()),
                    url.as_str(),
                ));
            }
        }
    }

    let css = Regex::new(r#"(?i)(?:url\(\s*["']?|@import\s+["'])([^"')\s]+)"#).unwrap();
    for piece in styles(files, id) {
        for c in css.captures_iter(&piece.text) {
            let url = c.get(1).unwrap();
            if is_outside(url.as_str()) {
                out.push(refusal(
                    &piece.path,
                    line_of(&piece.file, piece.start + url.start()),
                    url.as_str(),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(pairs: &[(&str, &str)]) -> Vec<String> {
        let mut f = Files::default();
        for (p, t) in pairs {
            f.add(p, t.as_bytes().to_vec());
        }
        let mut out = Vec::new();
        check(&f, "notes", &mut out);
        let mut told: Vec<String> = out.iter().map(Refusal::to_string).collect();
        told.sort();
        told
    }

    #[test]
    fn what_the_site_serves_passes() {
        assert_eq!(
            run(&[
                ("apps/notes/app.js", "import { app } from \"/_/yen.js\";\nimport './pages/home.js';\nfetch('/api/v1/me');"),
                (
                    "apps/notes/pages/home.js",
                    "// import 'https://cdn.example/x.js'\nhtml`<img src=\"/library/tiles/bell.svg\" alt=\"\"><a href=\"https://example.com\">a link</a>`",
                ),
                ("apps/notes/style.css", "a { background: url(\"/library/backgrounds/paper.png\"); }"),
            ]),
            Vec::<String>::new()
        );
    }

    #[test]
    fn what_comes_from_elsewhere_is_refused_on_its_line() {
        assert_eq!(
            run(&[
                ("apps/notes/app.js", "import { x } from \"https://cdn.example/x.js\";\nconst m = await import('//cdn.example/m.js');\nfetch(`https://api.example/v1`);"),
                ("apps/notes/pages/home.js", "html`\n<img alt=\"\" src=\"https://pics.example/a.png\">`"),
                ("apps/notes/index.html", "<head>\n<link rel=\"stylesheet\" href=\"https://fonts.example/css\">\n<style>@import 'https://x.example/a.css';</style>\n</head>"),
                ("apps/notes/style.css", "\n\na { background: url(http://pics.example/b.png); }"),
            ]),
            [
                "apps/notes/app.js:1 loads `https://cdn.example/x.js` from outside the site: everything an app loads is the site's own",
                "apps/notes/app.js:2 loads `//cdn.example/m.js` from outside the site: everything an app loads is the site's own",
                "apps/notes/app.js:3 loads `https://api.example/v1` from outside the site: everything an app loads is the site's own",
                "apps/notes/index.html:2 loads `https://fonts.example/css` from outside the site: everything an app loads is the site's own",
                "apps/notes/index.html:3 loads `https://x.example/a.css` from outside the site: everything an app loads is the site's own",
                "apps/notes/pages/home.js:2 loads `https://pics.example/a.png` from outside the site: everything an app loads is the site's own",
                "apps/notes/style.css:3 loads `http://pics.example/b.png` from outside the site: everything an app loads is the site's own",
            ]
        );
    }
}
