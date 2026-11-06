//! `guide`: an app is built to the app-building guide as it stands (§17.3).
//! A new app names the guide's edition in `guide:`; a changed app is brought
//! up to the current edition, which means reading the guide again. A tree
//! with no guide in it has no edition to hold an app to.

use crate::apps::manifest::Manifest;
use crate::check::files::{Files, GUIDE};
use crate::check::source::line_of;
use crate::check::{Before, Refusal};

use super::app_dir;

pub const RULE: &str = "guide";

pub fn check(files: &Files, id: &str, m: &Manifest, before: &Before, out: &mut Vec<Refusal>) {
    let Some(edition) = files.guide_edition() else {
        return;
    };
    let path = format!("{}app.yaml", app_dir(id));
    let line = files
        .text
        .get(&path)
        .and_then(|t| t.find("guide:").map(|at| line_of(t, at)));
    let message = match (m.guide, before.manifest.is_none()) {
        (None, true) => format!(
            "is a new app with no `guide:`: read {GUIDE} in full, then add `guide: {edition}`"
        ),
        (None, false) => format!(
            "has no `guide:`, and the guide is at edition {edition}: read {GUIDE} again, bring the app up to it, \
             and add `guide: {edition}`"
        ),
        (Some(g), _) if g < edition => format!(
            "is built to guide edition {g}, and the guide is at edition {edition}: read {GUIDE} again, bring \
             the app up to it, and set `guide: {edition}`"
        ),
        (Some(g), _) if g > edition => {
            format!("names guide edition {g}, but the guide is at edition {edition}")
        }
        _ => return,
    };
    out.push(Refusal::new(RULE, path, line, message));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(guide: Option<u32>, edition: Option<u32>, new: bool) -> Vec<String> {
        let yaml = match guide {
            Some(g) => format!("id: notes\nname: Notes\nguide: {g}\n"),
            None => "id: notes\nname: Notes\n".to_string(),
        };
        let mut f = Files::default();
        f.add("apps/notes/app.yaml", yaml.clone().into_bytes());
        if let Some(e) = edition {
            f.add(
                GUIDE,
                format!("# Building an app\n\nEdition: {e}\n").into_bytes(),
            );
        }
        let m = Manifest::parse(&yaml, "notes").unwrap();
        let was = m.clone();
        let before = Before {
            manifest: (!new).then_some(&was),
        };
        let mut out = Vec::new();
        check(&f, "notes", &m, &before, &mut out);
        out.iter().map(Refusal::to_string).collect()
    }

    #[test]
    fn an_app_built_to_the_current_guide_passes() {
        assert_eq!(run(Some(3), Some(3), true), Vec::<String>::new());
        assert_eq!(run(Some(3), Some(3), false), Vec::<String>::new());
        assert_eq!(run(None, None, true), Vec::<String>::new(), "no guide yet");
    }

    #[test]
    fn an_app_not_built_to_it_is_refused_with_what_to_do() {
        assert_eq!(
            run(None, Some(3), true),
            ["apps/notes/app.yaml is a new app with no `guide:`: read docs/building-an-app.md in full, then add `guide: 3`"]
        );
        assert_eq!(
            run(Some(2), Some(3), false),
            ["apps/notes/app.yaml:3 is built to guide edition 2, and the guide is at edition 3: read docs/building-an-app.md again, bring the app up to it, and set `guide: 3`"]
        );
        assert_eq!(
            run(None, Some(3), false),
            ["apps/notes/app.yaml has no `guide:`, and the guide is at edition 3: read docs/building-an-app.md again, bring the app up to it, and add `guide: 3`"]
        );
        assert_eq!(
            run(Some(4), Some(3), false),
            ["apps/notes/app.yaml:3 names guide edition 4, but the guide is at edition 3"]
        );
    }
}
