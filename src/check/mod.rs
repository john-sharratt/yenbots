//! The check (`candle/docs/yenbot_app_host.md` §17.4): the platform's rules
//! for an app, run on its files — by the gate on every candidate commit
//! ([`crate::gate`]), and on load for anything that arrived without one
//! ([`crate::apps`]). Every rule is deterministic: the same files always get
//! the same answer, and no rule asks a model anything.
//!
//! A rule either decides exactly (a file that is not there, a manifest that
//! does not parse, a component importing a store) or matches the source as
//! written (a literal colour, an `<img>` with no `alt`): what it refuses is
//! always wrong, though code assembled to hide from it could pass. What
//! cannot be decided from the source is left to the platform at run time —
//! the API refuses a field a collection does not declare on every write.
//!
//! The rules, one file each, under [`rules`]:
//!
//! | Rule | Refuses |
//! |---|---|
//! | `manifest` | a manifest that does not parse or is refused (names, fields, theme) |
//! | `references` | a tile, a schedule's workflow, a collection or a migration that is not there; a version lowered |
//! | `guide` | a new app without `guide:`, or a changed one built to an older edition |
//! | `layout` | a file outside the app's shape; a component outside its folder, two in a file, one not named for its tag, one reading a store |
//! | `tokens` | a literal colour, font or size where a token exists |
//! | `markup` | `unsafeHTML` and its kin, `innerHTML`, `eval`; an `<img>` without `alt`; an inline `style="…"`; a store's records drawn without `<yen-list>` |
//! | `outside` | a script, style, picture, font or call from outside the site |
//! | `workflows` | a workflow that does not compile, or exports no function |

pub mod files;
pub mod rules;
pub mod source;

use std::fmt;

use serde::Serialize;

use crate::apps::manifest::Manifest;
use files::Files;

/// One thing an app does that the platform refuses: which rule, where, and
/// what to do instead.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Refusal {
    /// The file, from the repository's root: `apps/recipes/pages/home.js`.
    pub path: String,
    /// The line, from 1, when the refusal is of one line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    pub rule: &'static str,
    pub message: String,
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(line) => write!(f, "{}:{line} {}", self.path, self.message),
            None => write!(f, "{} {}", self.path, self.message),
        }
    }
}

impl Refusal {
    pub fn new(
        rule: &'static str,
        path: impl Into<String>,
        line: Option<usize>,
        message: impl Into<String>,
    ) -> Refusal {
        Refusal {
            path: path.into(),
            line,
            rule,
            message: message.into(),
        }
    }
}

/// What the check knows of an app besides its files.
pub struct Before<'a> {
    /// The app's manifest as it was before this change — on `main` before a
    /// candidate, or the version being served before a reload; `None` for a
    /// new app.
    pub manifest: Option<&'a Manifest>,
}

/// Every refusal of app `id` in `files`, in order of path and line.
pub fn check_app(files: &Files, id: &str, before: &Before) -> Vec<Refusal> {
    let mut out = Vec::new();
    let manifest = rules::manifest::check(files, id, &mut out);
    if let Some(m) = &manifest {
        rules::references::check(files, id, m, before, &mut out);
        rules::guide::check(files, id, m, before, &mut out);
    }
    rules::layout::check(files, id, &mut out);
    rules::tokens::check(files, id, &mut out);
    rules::markup::check(files, id, &mut out);
    rules::outside::check(files, id, &mut out);
    rules::workflows::check(files, id, &mut out);
    out.sort();
    out.dedup();
    out
}

/// At most this many refusals are told in one answer; the rest are counted.
pub const TOLD: usize = 12;

/// The refusals as one answer, a line each.
pub fn told(refusals: &[Refusal]) -> String {
    let mut lines: Vec<String> = refusals.iter().take(TOLD).map(Refusal::to_string).collect();
    if refusals.len() > TOLD {
        lines.push(format!("… and {} more", refusals.len() - TOLD));
    }
    lines.join("\n")
}

#[cfg(test)]
mod templates_tests;
#[cfg(test)]
pub mod tests;
