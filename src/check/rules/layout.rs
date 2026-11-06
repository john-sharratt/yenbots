//! `layout`: every app has the one shape §8.3 gives it, so a new feature
//! adds files and never tangles one —
//!
//! ```text
//! app.yaml  index.html  app.js  style.css
//! pages/  components/  stores/  workflows/   JavaScript only
//! ```
//!
//! — and a component is defined in `pages/` or `components/`, one to a
//! file, a component's file named for its tag, and no component reads a
//! store: it takes its data as properties and raises events.

use regex::Regex;

use crate::check::files::Files;
use crate::check::source::line_of;
use crate::check::Refusal;

use super::{app_dir, scripts};

pub const RULE: &str = "layout";

const TOP: &[&str] = &["app.yaml", "index.html", "app.js", "style.css"];
const FOLDERS: &[&str] = &["pages", "components", "stores", "workflows"];

pub fn check(files: &Files, id: &str, out: &mut Vec<Refusal>) {
    let dir = app_dir(id);
    for path in files.names_under(&dir) {
        let rel = &path[dir.len()..];
        let allowed = match rel.split_once('/') {
            None => TOP.contains(&rel),
            Some((folder, _)) => FOLDERS.contains(&folder) && rel.ends_with(".js"),
        };
        if !allowed {
            out.push(Refusal::new(
                RULE,
                path,
                None,
                "is outside an app's shape: app.yaml, index.html, app.js and style.css, and JavaScript in \
                 pages/, components/, stores/ and workflows/",
            ));
        }
        if files.unreadable.contains(path) {
            out.push(Refusal::new(RULE, path, None, "is not UTF-8 text"));
        }
    }

    let defines = Regex::new(r#"customElements\s*\.\s*define\(\s*["'`]([^"'`]+)["'`]"#).unwrap();
    let imports = Regex::new(r#"(?:\bfrom\s*|\bimport\s*\(?\s*)["'`]([^"'`]+)["'`]"#).unwrap();
    for (path, js) in scripts(files, id) {
        let rel = &path[dir.len()..];
        let folder = rel.split_once('/').map(|(f, _)| f);
        for (n, c) in defines.captures_iter(&js).enumerate() {
            let tag = &c[1];
            let line = Some(line_of(&js, c.get(0).unwrap().start()));
            if !matches!(folder, Some("pages" | "components")) {
                out.push(Refusal::new(
                    RULE,
                    path.clone(),
                    line,
                    format!("defines <{tag}> outside pages/ and components/"),
                ));
            }
            if n > 0 {
                out.push(Refusal::new(
                    RULE,
                    path.clone(),
                    line,
                    format!("defines a second element, <{tag}>: one component to a file"),
                ));
            }
            if folder == Some("components") && rel != format!("components/{tag}.js") {
                out.push(Refusal::new(
                    RULE,
                    path.clone(),
                    line,
                    format!("defines <{tag}>; a component's file is named for its tag: components/{tag}.js"),
                ));
            }
        }
        if folder == Some("components") {
            for c in imports.captures_iter(&js) {
                if c[1].contains("stores/") {
                    out.push(Refusal::new(
                        RULE,
                        path.clone(),
                        Some(line_of(&js, c.get(1).unwrap().start())),
                        "reads a store: a component takes its data as properties and raises events, and its page \
                         reads the store",
                    ));
                }
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
    fn an_app_of_the_one_shape_passes() {
        assert_eq!(
            run(&[
                ("apps/notes/app.yaml", ""),
                ("apps/notes/index.html", ""),
                ("apps/notes/app.js", "import './pages/home.js';"),
                ("apps/notes/style.css", ""),
                ("apps/notes/pages/home.js", "import { notes } from '../stores/notes.js';\ncustomElements.define('notes-home', H);"),
                ("apps/notes/components/note-card.js", "customElements.define(\"note-card\", C);"),
                ("apps/notes/stores/notes.js", ""),
                ("apps/notes/workflows/remind.js", ""),
            ]),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_file_outside_the_shape_is_refused() {
        assert_eq!(
            run(&[("apps/notes/utils.js", ""), ("apps/notes/pages/home.css", "")]),
            [
                "apps/notes/pages/home.css is outside an app's shape: app.yaml, index.html, app.js and style.css, and JavaScript in pages/, components/, stores/ and workflows/",
                "apps/notes/utils.js is outside an app's shape: app.yaml, index.html, app.js and style.css, and JavaScript in pages/, components/, stores/ and workflows/",
            ]
        );
    }

    #[test]
    fn components_are_one_to_a_file_named_for_their_tag_and_read_no_store() {
        assert_eq!(
            run(&[
                ("apps/notes/app.js", "customElements.define('notes-app', A);"),
                (
                    "apps/notes/components/card.js",
                    "import { notes } from \"../stores/notes.js\";\n// customElements.define('in-a-comment', X);\ncustomElements.define('note-card', C);\ncustomElements.define('note-tag', T);",
                ),
            ]),
            [
                "apps/notes/app.js:1 defines <notes-app> outside pages/ and components/",
                "apps/notes/components/card.js:1 reads a store: a component takes its data as properties and raises events, and its page reads the store",
                "apps/notes/components/card.js:3 defines <note-card>; a component's file is named for its tag: components/note-card.js",
                "apps/notes/components/card.js:4 defines <note-tag>; a component's file is named for its tag: components/note-tag.js",
                "apps/notes/components/card.js:4 defines a second element, <note-tag>: one component to a file",
            ]
        );
    }

    #[test]
    fn a_file_that_is_not_text_is_refused() {
        let mut f = Files::default();
        f.add("apps/notes/style.css", vec![0xff]);
        let mut out = Vec::new();
        check(&f, "notes", &mut out);
        assert_eq!(
            out.iter().map(Refusal::to_string).collect::<Vec<_>>(),
            ["apps/notes/style.css is not UTF-8 text"]
        );
    }
}
