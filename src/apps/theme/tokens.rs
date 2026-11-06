//! The design tokens as the platform reads them (§8.5): `web/_/ui/tokens.css`,
//! the same file the browser loads, read once — so the contrast check judges
//! a theme against the surfaces the page will actually draw, and no colour is
//! written down twice.
//!
//! The file is three blocks in order: the light values on `:root`, the dark
//! values under `prefers-color-scheme: dark`, and the motion under
//! `prefers-reduced-motion`. A token the dark block leaves alone is the same
//! in both modes.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use anyhow::{anyhow, Result};
use regex::Regex;

use super::colour::Colour;

pub const SOURCE: &str = include_str!("../../../web/_/ui/tokens.css");

const DARK: &str = "@media (prefers-color-scheme: dark)";

#[derive(Debug, PartialEq, Eq)]
pub struct Tokens {
    pub light: BTreeMap<String, String>,
    pub dark: BTreeMap<String, String>,
}

/// `css` without its comments.
fn uncommented(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(at) = rest.find("/*") {
        out.push_str(&rest[..at]);
        rest = rest[at + 2..]
            .find("*/")
            .map_or("", |end| &rest[at + 2 + end + 2..]);
    }
    out.push_str(rest);
    out
}

/// Every `--yen-name: value;` in `css`, by name without the `--yen-`.
fn declarations(css: &str) -> BTreeMap<String, String> {
    static DECL: OnceLock<Regex> = OnceLock::new();
    let re = DECL.get_or_init(|| Regex::new(r"--yen-([a-z0-9-]+)\s*:\s*([^;]+);").unwrap());
    re.captures_iter(css)
        .map(|c| (c[1].to_string(), c[2].trim().to_string()))
        .collect()
}

impl Tokens {
    pub fn parse(css: &str) -> Tokens {
        let css = uncommented(css);
        let (light, rest) = css.split_once(DARK).unwrap_or((&css, ""));
        let dark = rest.split("@media").next().unwrap_or("");
        Tokens {
            light: declarations(light),
            dark: declarations(dark),
        }
    }

    /// The platform's own tokens.
    pub fn platform() -> &'static Tokens {
        static TOKENS: OnceLock<Tokens> = OnceLock::new();
        TOKENS.get_or_init(|| Tokens::parse(SOURCE))
    }

    /// The value of `--yen-<name>` in light or dark mode.
    pub fn get(&self, name: &str, dark: bool) -> Option<&str> {
        let dark_value = if dark { self.dark.get(name) } else { None };
        dark_value
            .or_else(|| self.light.get(name))
            .map(String::as_str)
    }

    /// `--yen-<name>` as a colour, following `var(--yen-…)` to the colour it names.
    pub fn colour(&self, name: &str, dark: bool) -> Result<Colour> {
        let mut at = name.to_string();
        for _ in 0..8 {
            let value = self
                .get(&at, dark)
                .ok_or_else(|| anyhow!("no token `--yen-{at}`"))?;
            if let Some(next) = value
                .strip_prefix("var(--yen-")
                .and_then(|v| v.strip_suffix(')'))
            {
                at = next.to_string();
                continue;
            }
            return Colour::parse(value)
                .ok_or_else(|| anyhow!("`--yen-{at}` is `{value}`, not a colour"));
        }
        Err(anyhow!("`--yen-{name}` names itself"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CSS: &str = "/* a: --yen-gone: #000; */\n:root {\n  --yen-bg: #fbf7f2;\n  --yen-page: var(--yen-bg);\n  \
                       --yen-accent: #c2375b; /* why */\n  --yen-loop: var(--yen-loop);\n}\n\
                       @media (prefers-color-scheme: dark) {\n  :root {\n    --yen-bg: #1a1613;\n  }\n}\n\
                       @media (prefers-reduced-motion: reduce) {\n  :root { --yen-duration-1: 0.01ms; }\n}\n";

    #[test]
    fn the_blocks_are_read_by_mode() {
        let t = Tokens::parse(CSS);
        assert_eq!(
            t.light.keys().collect::<Vec<_>>(),
            ["accent", "bg", "loop", "page"]
        );
        assert_eq!(t.dark.keys().collect::<Vec<_>>(), ["bg"]);
        assert_eq!(t.get("bg", false), Some("#fbf7f2"));
        assert_eq!(t.get("bg", true), Some("#1a1613"));
        assert_eq!(
            t.get("accent", true),
            Some("#c2375b"),
            "the same in both modes"
        );
        assert_eq!(t.get("duration-1", false), None, "motion is not a mode");
        assert_eq!(t.get("gone", false), None, "a comment is not a token");
    }

    #[test]
    fn a_colour_is_followed_through_its_names() {
        let t = Tokens::parse(CSS);
        assert_eq!(t.colour("page", false).unwrap().to_string(), "#fbf7f2");
        assert_eq!(t.colour("page", true).unwrap().to_string(), "#1a1613");
        assert!(t
            .colour("loop", false)
            .unwrap_err()
            .to_string()
            .contains("names itself"));
        assert!(t
            .colour("none", false)
            .unwrap_err()
            .to_string()
            .contains("no token"));
    }

    /// **The platform's tokens parse**: every surface has its six values in
    /// both modes, and the semantic names reach a colour.
    #[test]
    fn the_platforms_tokens_are_whole() {
        let t = Tokens::platform();
        for surface in ["warm", "plain", "cool", "rose", "sage"] {
            for part in [
                "bg",
                "surface",
                "surface-2",
                "surface-3",
                "line",
                "line-strong",
            ] {
                let name = format!("{surface}-{part}");
                assert!(t.light.contains_key(&name), "{name} light");
                assert!(t.dark.contains_key(&name), "{name} dark");
            }
        }
        for name in [
            "bg",
            "surface",
            "ink",
            "ink-2",
            "accent",
            "on-accent",
            "danger",
            "on-danger",
        ] {
            t.colour(name, false).unwrap();
            t.colour(name, true).unwrap();
        }
    }
}
