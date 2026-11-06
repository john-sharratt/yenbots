//! `tokens`: an app's styles are written in the design system's tokens
//! (§8.5), never a literal colour, font or size where a token exists — so
//! light and dark, a theme, and a change to the platform's look reach every
//! app without an edit. Read in every `.css` file, `css` template and
//! `<style>`.
//!
//! - **A colour** — `#d94f70`, `rgb(…)`, `hsl(…)`, `oklch(…)`, a named
//!   colour — in any declaration. `transparent`, `currentColor` and
//!   `inherit` are not colours of their own.
//! - **A size** — `12px`, `1.5rem`, `2em`, `10pt` — in a property the scale
//!   covers: spacing, corners, type and line height. A border or a width in
//!   pixels is no token's, and is not refused.
//! - **A font**: `font-family` or `font` naming a face rather than
//!   `var(--yen-font-…)`.

use regex::Regex;

use crate::check::files::Files;
use crate::check::source::{declarations, line_of};
use crate::check::Refusal;

use super::styles;

pub const RULE: &str = "tokens";

/// The CSS named colours an app might write.
const NAMED: &[&str] = &[
    "black",
    "white",
    "red",
    "green",
    "blue",
    "yellow",
    "orange",
    "purple",
    "pink",
    "brown",
    "gray",
    "grey",
    "silver",
    "gold",
    "navy",
    "teal",
    "maroon",
    "olive",
    "lime",
    "aqua",
    "cyan",
    "magenta",
    "fuchsia",
    "indigo",
    "violet",
    "coral",
    "salmon",
    "tomato",
    "crimson",
    "beige",
    "ivory",
    "khaki",
    "lavender",
    "plum",
    "orchid",
    "tan",
    "wheat",
    "chocolate",
    "turquoise",
    "skyblue",
    "steelblue",
    "royalblue",
    "darkblue",
    "darkgreen",
    "darkred",
    "darkgray",
    "darkgrey",
    "lightgray",
    "lightgrey",
    "lightblue",
    "lightgreen",
    "hotpink",
    "deeppink",
    "whitesmoke",
    "gainsboro",
    "slategray",
    "slategrey",
    "dimgray",
    "dimgrey",
    "rebeccapurple",
];

/// Which token a property's sizes come from, for the properties the scale
/// covers.
fn scale_of(property: &str) -> Option<&'static str> {
    let p = property;
    if p == "font-size" {
        Some("--yen-text-…")
    } else if p == "line-height" {
        Some("--yen-leading…")
    } else if p.contains("radius") {
        Some("--yen-radius-…")
    } else if p.starts_with("margin")
        || p.starts_with("padding")
        || p == "gap"
        || p == "row-gap"
        || p == "column-gap"
    {
        Some("--yen-space-…")
    } else {
        None
    }
}

pub fn check(files: &Files, id: &str, out: &mut Vec<Refusal>) {
    let hex = Regex::new(r"#(?:[0-9a-fA-F]{8}|[0-9a-fA-F]{6}|[0-9a-fA-F]{3,4})\b").unwrap();
    let function = Regex::new(r"(?i)\b(?:rgba?|hsla?|hwb|lab|lch|oklab|oklch|color)\s*\(").unwrap();
    let named = Regex::new(&format!(
        r"(?i)(?:^|[\s,(])({})(?:$|[\s,);!])",
        NAMED.join("|")
    ))
    .unwrap();
    let size = Regex::new(r"(?i)(?:^|[^\w.#-])(\d*\.?\d+)(px|rem|em|pt)\b").unwrap();

    for piece in styles(files, id) {
        for d in declarations(&piece.text) {
            let line = |offset: usize| Some(line_of(&piece.file, piece.start + d.at + offset));
            let mut colour = None;
            if let Some(m) = hex.find(&d.value) {
                colour = Some((m.start(), m.as_str().to_string()));
            } else if let Some(m) = function.find(&d.value) {
                let end = d.value[m.start()..]
                    .find(')')
                    .map_or(d.value.len(), |e| m.start() + e + 1);
                colour = Some((m.start(), d.value[m.start()..end].to_string()));
            } else if let Some(c) = named.captures(&d.value) {
                let m = c.get(1).unwrap();
                colour = Some((m.start(), m.as_str().to_string()));
            }
            if let Some((at, written)) = colour {
                out.push(Refusal::new(
                    RULE,
                    piece.path.clone(),
                    line(at),
                    format!("uses the colour `{written}`; use a token: var(--yen-accent), var(--yen-ink), …"),
                ));
            }
            if let Some(token) = scale_of(&d.property) {
                if let Some(c) = size.captures(&d.value) {
                    let m = c.get(1).unwrap();
                    let written = format!("{}{}", &c[1], &c[2]);
                    let zero = m.as_str().parse::<f64>().is_ok_and(|n| n == 0.0);
                    if !zero {
                        out.push(Refusal::new(
                            RULE,
                            piece.path.clone(),
                            line(m.start()),
                            format!(
                                "gives {} the size `{written}`; use a token: var({token})",
                                d.property
                            ),
                        ));
                    }
                }
            }
            let names_a_face = (d.property == "font-family" || d.property == "font")
                && !d.value.contains("var(--yen-font")
                && !matches!(d.value.as_str(), "inherit" | "initial" | "unset");
            if names_a_face {
                out.push(Refusal::new(
                    RULE,
                    piece.path.clone(),
                    line(0),
                    format!(
                        "sets {} to `{}`; use var(--yen-font-text) or var(--yen-font-heading)",
                        d.property, d.value
                    ),
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
    fn styles_in_tokens_pass() {
        assert_eq!(
            run(&[
                (
                    "apps/notes/style.css",
                    "/* #fff in a comment */\n#fade-in { color: var(--yen-ink); margin: 0 auto; padding: calc(var(--yen-space-2) * 2); }\n\
                     .x { border: 1px solid var(--yen-line); width: 320px; background: transparent; font: inherit; }",
                ),
                (
                    "apps/notes/components/note-card.js",
                    "static styles = css`:host { gap: var(--yen-space-3); font-family: var(--yen-font-heading); }`;",
                ),
            ]),
            Vec::<String>::new()
        );
    }

    /// **A literal colour, size or face is refused on its line, with the
    /// token to use** — in a stylesheet, a `css` template and a `<style>`.
    #[test]
    fn literals_are_refused_where_they_are() {
        assert_eq!(
            run(&[
                ("apps/notes/style.css", "a {\n  color: #D94F70;\n  margin: 12px 0;\n}\n.b { background: rgb(1 2 3); font-size: 1.2rem }"),
                (
                    "apps/notes/components/note-card.js",
                    "// note\nstatic styles = css`\n  :host {\n    border-radius: 8px;\n    color: white;\n    font-family: Georgia, serif;\n  }\n`;",
                ),
                ("apps/notes/index.html", "<head>\n<style>\nbody { --yen-accent: #ff0000; }\n</style>\n</head>"),
            ]),
            [
                "apps/notes/components/note-card.js:4 gives border-radius the size `8px`; use a token: var(--yen-radius-…)",
                "apps/notes/components/note-card.js:5 uses the colour `white`; use a token: var(--yen-accent), var(--yen-ink), …",
                "apps/notes/components/note-card.js:6 sets font-family to `Georgia, serif`; use var(--yen-font-text) or var(--yen-font-heading)",
                "apps/notes/index.html:3 uses the colour `#ff0000`; use a token: var(--yen-accent), var(--yen-ink), …",
                "apps/notes/style.css:2 uses the colour `#D94F70`; use a token: var(--yen-accent), var(--yen-ink), …",
                "apps/notes/style.css:3 gives margin the size `12px`; use a token: var(--yen-space-…)",
                "apps/notes/style.css:5 gives font-size the size `1.2rem`; use a token: var(--yen-text-…)",
                "apps/notes/style.css:5 uses the colour `rgb(1 2 3)`; use a token: var(--yen-accent), var(--yen-ink), …",
            ]
        );
    }
}
