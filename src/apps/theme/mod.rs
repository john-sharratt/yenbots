//! An app's own look (§8.5): the few named tokens its manifest may set under
//! `theme:`, checked so an override can change the app's character but never
//! its legibility, and written as the stylesheet the site serves for the app
//! (`/_/theme/<id>.css`), after the platform's.
//!
//! ```yaml
//! theme:
//!   accent: "#b03a5b"        # the app's colour: buttons, links, its tile
//!   accent-dark: "#f07a95"   # the same in dark mode; lightened from `accent` when absent
//!   surface: warm            # warm | plain | cool | rose | sage
//!   radius: round            # soft | round | square
//!   type: display            # text | display — the face for headings
//!   density: roomy           # compact | normal | roomy
//! ```
//!
//! Only these names exist. The accent must read at WCAG AA (4.5:1) against
//! the page and a card of its surface, in light mode and in dark; the ink on
//! a button is whichever of white and the dark page reads better on it.

pub mod colour;
pub mod tokens;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use colour::{contrast, Colour};
use tokens::Tokens;

/// The contrast ratio text needs to be read: WCAG AA for body text.
pub const AA: f64 = 4.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Surface {
    Warm,
    Plain,
    Cool,
    Rose,
    Sage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Radius {
    Soft,
    Round,
    Square,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Face {
    Text,
    Display,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Density {
    Compact,
    Normal,
    Roomy,
}

impl Surface {
    fn name(self) -> &'static str {
        match self {
            Surface::Warm => "warm",
            Surface::Plain => "plain",
            Surface::Cool => "cool",
            Surface::Rose => "rose",
            Surface::Sage => "sage",
        }
    }
}

impl Radius {
    fn name(self) -> &'static str {
        match self {
            Radius::Soft => "soft",
            Radius::Round => "round",
            Radius::Square => "square",
        }
    }
}

impl Face {
    fn name(self) -> &'static str {
        match self {
            Face::Text => "text",
            Face::Display => "display",
        }
    }
}

impl Density {
    fn name(self) -> &'static str {
        match self {
            Density::Compact => "compact",
            Density::Normal => "normal",
            Density::Roomy => "roomy",
        }
    }
}

/// The parts of a surface the theme points the page at.
const SURFACE_PARTS: &[&str] = &[
    "bg",
    "surface",
    "surface-2",
    "surface-3",
    "line",
    "line-strong",
];

/// The corners a radius sets.
const RADIUS_PARTS: &[&str] = &["1", "2", "3", "button"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Theme {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent: Option<Colour>,
    #[serde(
        default,
        rename = "accent-dark",
        skip_serializing_if = "Option::is_none"
    )]
    pub accent_dark: Option<Colour>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface: Option<Surface>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius: Option<Radius>,
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub face: Option<Face>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub density: Option<Density>,
}

/// One mode's accent and the ink on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Accent {
    pub accent: Colour,
    pub on: Colour,
}

fn mode(dark: bool) -> &'static str {
    if dark {
        "dark"
    } else {
        "light"
    }
}

impl Theme {
    fn surface_name(&self) -> &'static str {
        self.surface.unwrap_or(Surface::Warm).name()
    }

    /// The worse of the accent's contrast against the page and a card.
    fn reads(&self, t: &Tokens, accent: Colour, dark: bool) -> Result<f64> {
        let s = self.surface_name();
        let page = t.colour(&format!("{s}-bg"), dark)?;
        let card = t.colour(&format!("{s}-surface"), dark)?;
        Ok(contrast(accent, page).min(contrast(accent, card)))
    }

    /// The accent of one mode as written, derived, or the platform's.
    fn accent_of(&self, t: &Tokens, dark: bool) -> Result<Colour> {
        match (dark, self.accent, self.accent_dark) {
            (false, Some(a), _) | (true, _, Some(a)) => Ok(a),
            // Lightened, a twentieth at a time, until it reads on the dark page.
            (true, Some(a), None) => {
                for step in 0..=20 {
                    let lighter = a.mix(Colour::WHITE, f64::from(step) / 20.0);
                    if self.reads(t, lighter, true)? >= AA {
                        return Ok(lighter);
                    }
                }
                Ok(Colour::WHITE)
            }
            _ => t.colour("accent", dark),
        }
    }

    /// The accent and the ink on it, in light or dark mode.
    pub fn accent(&self, t: &Tokens, dark: bool) -> Result<Accent> {
        let accent = self.accent_of(t, dark)?;
        let ink = t.colour("warm-bg", true)?;
        let on = if contrast(Colour::WHITE, accent) >= contrast(ink, accent) {
            Colour::WHITE
        } else {
            ink
        };
        Ok(Accent { accent, on })
    }

    /// Whether the theme can be read in both modes; why not, when it cannot.
    pub fn check(&self, t: &Tokens) -> Result<()> {
        for dark in [false, true] {
            let a = self.accent(t, dark)?;
            let ratio = self.reads(t, a.accent, dark)?;
            if ratio < AA {
                let way = if dark { "lighter" } else { "deeper" };
                let written = if dark && self.accent_dark.is_some() {
                    "accent-dark"
                } else {
                    "accent"
                };
                bail!(
                    "theme: the {written} {} is {ratio:.2}:1 against the {} surface in {} mode, and text needs \
                     {AA}:1 to be read — choose a {way} colour",
                    a.accent,
                    self.surface_name(),
                    mode(dark),
                );
            }
            let on = contrast(a.on, a.accent);
            if on < AA {
                bail!(
                    "theme: no ink reads on the accent {} in {} mode (the best is {on:.2}:1) — choose a deeper \
                     or a lighter colour",
                    a.accent,
                    mode(dark),
                );
            }
        }
        Ok(())
    }

    /// The stylesheet that gives the app this theme, after the platform's.
    pub fn css(&self, t: &Tokens) -> Result<String> {
        let mut root = Vec::new();
        if let Some(s) = self.surface {
            for part in SURFACE_PARTS {
                root.push(format!("--yen-{part}: var(--yen-{}-{part});", s.name()));
            }
        }
        if let Some(r) = self.radius {
            for part in RADIUS_PARTS {
                root.push(format!(
                    "--yen-radius-{part}: var(--yen-radius-{}-{part});",
                    r.name()
                ));
            }
        }
        if let Some(f) = self.face {
            root.push(format!("--yen-font-heading: var(--yen-font-{});", f.name()));
        }
        if let Some(d) = self.density {
            root.push(format!("--yen-density: var(--yen-density-{});", d.name()));
        }
        let mut dark = Vec::new();
        if self.accent.is_some() {
            let a = self.accent(t, false)?;
            root.push(format!("--yen-accent: {};", a.accent));
            root.push(format!("--yen-on-accent: {};", a.on));
        }
        if self.accent.is_some() || self.accent_dark.is_some() {
            let a = self.accent(t, true)?;
            dark.push(format!("--yen-accent: {};", a.accent));
            dark.push(format!("--yen-on-accent: {};", a.on));
        }
        let block = |lines: &[String], indent: &str| {
            lines
                .iter()
                .map(|l| format!("{indent}{l}\n"))
                .collect::<String>()
        };
        let mut css = String::from("/* This app's theme, from its manifest. */\n");
        if !root.is_empty() {
            css.push_str(&format!(":root {{\n{}}}\n", block(&root, "  ")));
        }
        if !dark.is_empty() {
            css.push_str(&format!(
                "@media (prefers-color-scheme: dark) {{\n  :root {{\n{}  }}\n}}\n",
                block(&dark, "    ")
            ));
        }
        Ok(css)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn theme(yaml: &str) -> Theme {
        serde_yaml::from_str(yaml).unwrap()
    }

    fn platform() -> &'static Tokens {
        Tokens::platform()
    }

    /// **The platform's own palette reads**: the house accent, the ink and
    /// the status colours reach AA on every surface, in both modes, and so
    /// does the ink on the accent and on danger.
    #[test]
    fn the_house_palette_is_legible_on_every_surface() {
        let t = platform();
        for dark in [false, true] {
            for s in ["warm", "plain", "cool", "rose", "sage"] {
                for ground in ["bg", "surface", "surface-2"] {
                    let under = t.colour(&format!("{s}-{ground}"), dark).unwrap();
                    for ink in ["ink", "ink-2", "accent", "danger", "success", "warning"] {
                        let ratio = contrast(t.colour(ink, dark).unwrap(), under);
                        let needed = if ink == "accent" && ground == "surface-2" {
                            0.0
                        } else {
                            AA
                        };
                        assert!(
                            ratio >= needed,
                            "{ink} on {s}-{ground} ({}): {ratio:.2}",
                            mode(dark)
                        );
                    }
                }
            }
            for (ink, on) in [("on-accent", "accent"), ("on-danger", "danger")] {
                let ratio = contrast(t.colour(ink, dark).unwrap(), t.colour(on, dark).unwrap());
                assert!(ratio >= AA, "{ink} on {on} ({}): {ratio:.2}", mode(dark));
            }
            assert!(Theme::default().check(t).is_ok());
        }
    }

    #[test]
    fn a_theme_takes_only_its_names() {
        let t = theme("accent: \"#b03a5b\"\naccent-dark: \"#f07a95\"\nsurface: cool\nradius: round\ntype: text\ndensity: roomy\n");
        assert_eq!(t.surface, Some(Surface::Cool));
        assert_eq!(t.face, Some(Face::Text));
        assert_eq!(t.accent.unwrap().to_string(), "#b03a5b");
        let unknown = serde_yaml::from_str::<Theme>("colour: pink\n")
            .unwrap_err()
            .to_string();
        assert!(unknown.contains("unknown field `colour`"), "{unknown}");
        let named = serde_yaml::from_str::<Theme>("accent: pink\n")
            .unwrap_err()
            .to_string();
        assert!(named.contains("`pink` is not a colour"), "{named}");
        assert!(serde_yaml::from_str::<Theme>("radius: pointy\n").is_err());
    }

    /// **An accent too pale to read is refused, saying how far off it is and
    /// which way to go**; one that reads is taken, in both modes.
    #[test]
    fn an_accent_must_read_in_both_modes() {
        let t = platform();
        let pale = theme("accent: \"#f9a8c0\"\naccent-dark: \"#f9a8c0\"\n")
            .check(t)
            .unwrap_err()
            .to_string();
        assert_eq!(
            pale,
            "theme: the accent #f9a8c0 is 1.73:1 against the warm surface in light mode, and text needs 4.5:1 to \
             be read — choose a deeper colour"
        );
        let dim = theme("accent: \"#b03a5b\"\naccent-dark: \"#8a2040\"\n")
            .check(t)
            .unwrap_err()
            .to_string();
        assert!(
            dim.starts_with("theme: the accent-dark #8a2040 is"),
            "{dim}"
        );
        assert!(
            dim.ends_with(
                "in dark mode, and text needs 4.5:1 to be read — choose a lighter colour"
            ),
            "{dim}"
        );
        assert!(theme("accent: \"#b03a5b\"\naccent-dark: \"#f07a95\"\n")
            .check(t)
            .is_ok());
    }

    /// **Without `accent-dark`, the dark accent is the light one lightened
    /// until it reads on the dark page.**
    #[test]
    fn the_dark_accent_is_derived_when_absent() {
        let t = platform();
        let th = theme("accent: \"#b03a5b\"\n");
        let dark = th.accent(t, true).unwrap();
        // A quarter of the way to white: 4.55:1 against the dark card.
        assert_eq!(dark.accent.to_string(), "#c46b84");
        assert!(th.reads(t, dark.accent, true).unwrap() >= AA);
        let one_less = Colour::parse("#b03a5b").unwrap().mix(Colour::WHITE, 0.2);
        assert!(
            th.reads(t, one_less, true).unwrap() < AA,
            "the first that reads, no lighter"
        );
        assert!(th.check(t).is_ok());
    }

    #[test]
    fn the_ink_on_the_accent_is_whichever_reads_better() {
        let t = platform();
        let light = theme("accent: \"#b03a5b\"\n").accent(t, false).unwrap();
        assert_eq!(light.on, Colour::WHITE);
        let dark = theme("accent: \"#b03a5b\"\n").accent(t, true).unwrap();
        assert_eq!(dark.on.to_string(), "#1a1613");
    }

    /// **The stylesheet points the page at named tokens**, and every name it
    /// points at is one the platform's tokens define.
    #[test]
    fn a_theme_is_written_as_named_tokens() {
        let t = platform();
        let css = theme("accent: \"#b03a5b\"\naccent-dark: \"#f07a95\"\nsurface: cool\nradius: round\ntype: text\ndensity: roomy\n")
            .css(t)
            .unwrap();
        assert_eq!(
            css,
            "/* This app's theme, from its manifest. */\n\
             :root {\n\
             \x20 --yen-bg: var(--yen-cool-bg);\n\
             \x20 --yen-surface: var(--yen-cool-surface);\n\
             \x20 --yen-surface-2: var(--yen-cool-surface-2);\n\
             \x20 --yen-surface-3: var(--yen-cool-surface-3);\n\
             \x20 --yen-line: var(--yen-cool-line);\n\
             \x20 --yen-line-strong: var(--yen-cool-line-strong);\n\
             \x20 --yen-radius-1: var(--yen-radius-round-1);\n\
             \x20 --yen-radius-2: var(--yen-radius-round-2);\n\
             \x20 --yen-radius-3: var(--yen-radius-round-3);\n\
             \x20 --yen-radius-button: var(--yen-radius-round-button);\n\
             \x20 --yen-font-heading: var(--yen-font-text);\n\
             \x20 --yen-density: var(--yen-density-roomy);\n\
             \x20 --yen-accent: #b03a5b;\n\
             \x20 --yen-on-accent: #ffffff;\n\
             }\n\
             @media (prefers-color-scheme: dark) {\n\
             \x20 :root {\n\
             \x20   --yen-accent: #f07a95;\n\
             \x20   --yen-on-accent: #1a1613;\n\
             \x20 }\n\
             }\n"
        );
        assert_eq!(
            Theme::default().css(t).unwrap(),
            "/* This app's theme, from its manifest. */\n"
        );
        // Every variant's names exist.
        for s in ["warm", "plain", "cool", "rose", "sage"] {
            for r in ["soft", "round", "square"] {
                for f in ["text", "display"] {
                    for d in ["compact", "normal", "roomy"] {
                        let css = theme(&format!(
                            "surface: {s}\nradius: {r}\ntype: {f}\ndensity: {d}\n"
                        ))
                        .css(t)
                        .unwrap();
                        for name in css
                            .split("var(--yen-")
                            .skip(1)
                            .map(|s| s.split(')').next().unwrap())
                        {
                            assert!(t.light.contains_key(name), "{name}");
                        }
                    }
                }
            }
        }
    }
}
