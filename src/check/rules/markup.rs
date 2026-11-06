//! `markup`: what an app draws is escaped, described and in the platform's
//! components (§8.5, §12.6).
//!
//! - **Escaped**: no `unsafeHTML`, `unsafeSVG`, `unsafeStatic` or
//!   `unsafeCSS`; no markup written from a string (`innerHTML =`,
//!   `outerHTML =`, `insertAdjacentHTML`, `document.write`); no code run
//!   from a string (`eval`, `new Function`), which the content policy
//!   refuses anyway.
//! - **Described**: every `<img>` has an `alt` — what it shows, or `alt=""`
//!   for a picture that only decorates.
//! - **Styled by class**: no inline `style="…"`, which the content policy
//!   refuses; `style=${styleMap(…)}` sets properties, and is not one.
//! - **Lists in `<yen-list>`**: a store's records (`recipes.value`) drawn
//!   with `.map` in a template have no loading, empty or error state;
//!   `<yen-list>` has all three.

use regex::Regex;

use crate::check::files::Files;
use crate::check::source::line_of;
use crate::check::Refusal;

use super::{app_dir, markup, scripts};

pub const RULE: &str = "markup";

pub fn check(files: &Files, id: &str, out: &mut Vec<Refusal>) {
    let checks: [(Regex, &str); 3] = [
        (
            Regex::new(r"\b(unsafeHTML|unsafeSVG|unsafeStatic|unsafeCSS)\b").unwrap(),
            "uses `{}`: an app's markup is always escaped; render values with ${…}",
        ),
        (
            Regex::new(r"(\.innerHTML\s*=[^=]|\.outerHTML\s*=[^=]|\binsertAdjacentHTML\s*\(|\bdocument\s*\.\s*write\s*\()").unwrap(),
            "writes markup from a string (`{}`); render it with html`…`",
        ),
        (
            Regex::new(r"(\beval\s*\(|\bnew\s+Function\s*\()").unwrap(),
            "runs code from a string (`{}`), which the site's content policy refuses",
        ),
    ];
    for (path, js) in scripts(files, id) {
        for (re, says) in &checks {
            for c in re.captures_iter(&js) {
                let m = c.get(1).unwrap();
                let written = m
                    .as_str()
                    .trim_end_matches(|ch: char| !ch.is_alphanumeric() && ch != '(' && ch != ')');
                out.push(Refusal::new(
                    RULE,
                    path.clone(),
                    Some(line_of(&js, m.start())),
                    says.replace("{}", written.trim()),
                ));
            }
        }
    }

    let img = Regex::new(r"(?is)<img\b[^>]*>").unwrap();
    let alt = Regex::new(r"(?i)\salt\s*=").unwrap();
    let style = Regex::new(r#"(?i)\sstyle\s*=\s*["']"#).unwrap();
    let mapped = Regex::new(r"\.value\s*\.\s*map\s*\(").unwrap();
    let dir = app_dir(id);
    for piece in markup(files, id) {
        let at = |offset: usize| Some(line_of(&piece.file, piece.start + offset));
        for m in img.find_iter(&piece.text) {
            if !alt.is_match(m.as_str()) {
                out.push(Refusal::new(
                    RULE,
                    piece.path.clone(),
                    at(m.start()),
                    "has an <img> with no alt: say what it shows, or alt=\"\" for a picture that only decorates",
                ));
            }
        }
        for m in style.find_iter(&piece.text) {
            out.push(Refusal::new(
                RULE,
                piece.path.clone(),
                at(m.start() + 1),
                "has an inline style=\"…\", which the site's content policy refuses: use a class in the \
                 component's styles, or style=${styleMap(…)}",
            ));
        }
        let drawn = piece.path[dir.len()..].starts_with("pages/")
            || piece.path[dir.len()..].starts_with("components/");
        if drawn {
            for m in mapped.find_iter(&piece.raw) {
                out.push(Refusal::new(
                    RULE,
                    piece.path.clone(),
                    at(m.start()),
                    "draws a store's records with .map: use <yen-list>, which shows their loading, empty and error \
                     states",
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
        told.dedup();
        told
    }

    #[test]
    fn markup_that_is_escaped_described_and_listed_passes() {
        assert_eq!(
            run(&[(
                "apps/notes/pages/home.js",
                "// unsafeHTML in a comment\nrender() { return html`<img src=${p} alt=\"\"><div style=${styleMap(s)}></div>\
                 <yen-list .items=${notes.value} .item=${(n) => html`<p>${n.tags.map((t) => t)}</p>`}></yen-list>`; }\n\
                 const same = a.innerHTML === b;",
            )]),
            Vec::<String>::new()
        );
    }

    #[test]
    fn markup_that_is_not_is_refused_on_its_line() {
        assert_eq!(
            run(&[
                (
                    "apps/notes/pages/home.js",
                    "import { unsafeHTML } from '/_/yen.js';\nel.innerHTML = text;\neval(code);\n\
                     render() {\n  return html`\n    <img src=${p}>\n    <p style=\"color: red\">${notes.value.map((n) => n.text)}</p>`;\n}",
                ),
                ("apps/notes/index.html", "<body>\n<img src=\"a.png\">\n</body>"),
            ]),
            [
                "apps/notes/index.html:2 has an <img> with no alt: say what it shows, or alt=\"\" for a picture that only decorates",
                "apps/notes/pages/home.js:1 uses `unsafeHTML`: an app's markup is always escaped; render values with ${…}",
                "apps/notes/pages/home.js:2 writes markup from a string (`.innerHTML`); render it with html`…`",
                "apps/notes/pages/home.js:3 runs code from a string (`eval(`), which the site's content policy refuses",
                "apps/notes/pages/home.js:6 has an <img> with no alt: say what it shows, or alt=\"\" for a picture that only decorates",
                "apps/notes/pages/home.js:7 draws a store's records with .map: use <yen-list>, which shows their loading, empty and error states",
                "apps/notes/pages/home.js:7 has an inline style=\"…\", which the site's content policy refuses: use a class in the component's styles, or style=${styleMap(…)}",
            ]
        );
    }
}
