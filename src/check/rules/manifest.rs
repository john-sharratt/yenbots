//! `manifest`: an app has an `app.yaml`, and the platform takes it — it
//! parses, its names are names, its fields and its theme are as §5.1, §7.5
//! and §8.5 say ([`Manifest::parse`]).

use crate::apps::manifest::Manifest;
use crate::check::files::Files;
use crate::check::Refusal;

use super::app_dir;

pub const RULE: &str = "manifest";

/// The line a YAML error names, when one of its causes is a YAML error.
fn yaml_line(e: &anyhow::Error) -> Option<usize> {
    e.chain()
        .find_map(|c| c.downcast_ref::<serde_yaml::Error>())
        .and_then(|y| y.location())
        .map(|l| l.line())
}

/// App `id`'s manifest, when the platform takes it; a refusal when it does
/// not, or when an app folder has none.
pub fn check(files: &Files, id: &str, out: &mut Vec<Refusal>) -> Option<Manifest> {
    let path = format!("{}app.yaml", app_dir(id));
    let Some(text) = files.text.get(&path) else {
        if files.names_under(&app_dir(id)).next().is_some() {
            out.push(Refusal::new(
                RULE,
                path,
                None,
                "is missing: every app folder has its manifest, app.yaml",
            ));
        }
        return None;
    };
    match Manifest::parse(text, id) {
        Ok(m) => Some(m),
        Err(e) => {
            out.push(Refusal::new(RULE, path, yaml_line(&e), format!("{e:#}")));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(pairs: &[(&str, &str)]) -> Files {
        let mut f = Files::default();
        for (p, t) in pairs {
            f.add(p, t.as_bytes().to_vec());
        }
        f
    }

    #[test]
    fn a_manifest_the_platform_takes_is_read() {
        let mut out = Vec::new();
        let m = check(
            &files(&[("apps/notes/app.yaml", "id: notes\nname: Notes\n")]),
            "notes",
            &mut out,
        );
        assert_eq!(m.unwrap().name, "Notes");
        assert_eq!(out, []);
    }

    #[test]
    fn a_manifest_refused_is_said_where_and_why() {
        let mut out = Vec::new();
        let f = files(&[("apps/notes/app.yaml", "id: notes\nname: Notes\ncollections:\n  n:\n    fields:\n      version: { type: text }\n")]);
        assert_eq!(check(&f, "notes", &mut out), None);
        assert_eq!(
            out,
            [Refusal::new(
                RULE,
                "apps/notes/app.yaml",
                None,
                "n: `version` is the record's own; call the field something else"
            )]
        );
        let mut out = Vec::new();
        check(
            &files(&[("apps/notes/app.yaml", "id: notes\nname: [\n")]),
            "notes",
            &mut out,
        );
        assert_eq!(out[0].line, Some(2), "the line the unclosed `[` opens on");
        assert!(
            out[0].message.starts_with("app.yaml does not parse: "),
            "{}",
            out[0].message
        );
    }

    #[test]
    fn an_app_folder_without_a_manifest_is_refused() {
        let mut out = Vec::new();
        check(
            &files(&[("apps/notes/app.js", "export {};")]),
            "notes",
            &mut out,
        );
        assert_eq!(
            out,
            [Refusal::new(
                RULE,
                "apps/notes/app.yaml",
                None,
                "is missing: every app folder has its manifest, app.yaml"
            )]
        );
        let mut none = Vec::new();
        assert_eq!(check(&Files::default(), "notes", &mut none), None);
        assert_eq!(none, [], "an app that is gone is not refused");
    }
}
