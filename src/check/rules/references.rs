//! `references`: everything an app names is there — its tile in the
//! library, each schedule's workflow, each collection its code opens — and a
//! collection's version only rises, each step with the migration that takes
//! a record up it (§7.5).

use regex::Regex;

use crate::apps::manifest::Manifest;
use crate::check::files::Files;
use crate::check::source::line_of;
use crate::check::{Before, Refusal};

use super::{app_dir, scripts};

pub const RULE: &str = "references";

/// The extensions a library picture may have.
const PICTURES: &[&str] = &["svg", "png", "webp", "jpg", "jpeg"];

/// The line of `app.yaml` first holding `needle`, if it does.
fn line_in(text: &str, needle: &str) -> Option<usize> {
    text.find(needle).map(|at| line_of(text, at))
}

pub fn check(files: &Files, id: &str, m: &Manifest, before: &Before, out: &mut Vec<Refusal>) {
    let dir = app_dir(id);
    let yaml_path = format!("{dir}app.yaml");
    let yaml = files.text.get(&yaml_path).map(String::as_str).unwrap_or("");
    let refuse = |out: &mut Vec<Refusal>, needle: &str, message: String| {
        out.push(Refusal::new(
            RULE,
            yaml_path.clone(),
            line_in(yaml, needle),
            message,
        ));
    };

    if let Some(tile) = &m.tile {
        let there = tile.starts_with("library/")
            && (files.has(tile) || PICTURES.iter().any(|e| files.has(&format!("{tile}.{e}"))));
        if !there {
            refuse(
                out,
                "tile:",
                format!("names the tile `{tile}`, which is not in the art library"),
            );
        }
    }

    for s in &m.schedules {
        if !files.has(&format!("{dir}workflows/{}.js", s.run)) {
            refuse(
                out,
                &format!("run: {}", s.run),
                format!(
                    "a schedule runs `{}`, but there is no workflows/{}.js",
                    s.run, s.run
                ),
            );
        }
    }

    let opened = Regex::new(r#"\.collection\(\s*["'`]([^"'`]+)["'`]"#).unwrap();
    for (path, js) in scripts(files, id) {
        for c in opened.captures_iter(&js) {
            let name = &c[1];
            if !m.collections.contains_key(name) {
                out.push(Refusal::new(
                    RULE,
                    path.clone(),
                    Some(line_of(&js, c.get(1).unwrap().start())),
                    format!("opens the collection `{name}`, which app.yaml does not declare"),
                ));
            }
        }
    }

    let Some(was) = before.manifest else { return };
    for (name, c) in &m.collections {
        let Some(old) = was.collections.get(name) else {
            continue;
        };
        if c.version < old.version {
            refuse(
                out,
                &format!("{name}:"),
                format!(
                    "lowers {name}'s version from {} to {}; a version only rises",
                    old.version, c.version
                ),
            );
            continue;
        }
        for n in old.version + 1..=c.version {
            let migration = format!("workflows/migrate-{name}-{n}.js");
            if !files.has(&format!("{dir}{migration}")) {
                refuse(
                    out,
                    &format!("{name}:"),
                    format!(
                        "raises {name} to version {n} without {migration}, which turns a record at version {} into one at {n}",
                        n - 1
                    ),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const YAML: &str = "id: notes\nname: Notes\ntile: library/tiles/bell\ncollections:\n  notes:\n    version: 3\n    fields:\n      text: { type: text }\nschedules:\n  - every: daily 07:30\n    run: morning\n";

    fn files(extra: &[(&str, &str)]) -> Files {
        let mut f = Files::default();
        f.add("apps/notes/app.yaml", YAML.as_bytes().to_vec());
        for (p, t) in extra {
            f.add(p, t.as_bytes().to_vec());
        }
        f
    }

    fn run(f: &Files, before: Option<&Manifest>) -> Vec<String> {
        let m = Manifest::parse(f.text["apps/notes/app.yaml"].as_str(), "notes").unwrap();
        let mut out = Vec::new();
        check(f, "notes", &m, &Before { manifest: before }, &mut out);
        out.iter().map(Refusal::to_string).collect()
    }

    fn was(version: u32) -> Manifest {
        Manifest::parse(
            &YAML.replace("version: 3", &format!("version: {version}")),
            "notes",
        )
        .unwrap()
    }

    #[test]
    fn what_an_app_names_is_there() {
        let mut f = files(&[
            (
                "apps/notes/workflows/morning.js",
                "export default async function m() {}",
            ),
            (
                "apps/notes/stores/notes.js",
                "export const notes = app.collection(\"notes\");",
            ),
        ]);
        f.name("library/tiles/bell.svg");
        assert_eq!(run(&f, None), Vec::<String>::new());
    }

    #[test]
    fn what_is_not_there_is_refused_on_its_line() {
        let f = files(&[(
            "apps/notes/stores/notes.js",
            "// app.collection(\"commented\")\nexport const notes = app.collection('notez');",
        )]);
        assert_eq!(
            run(&f, None),
            [
                "apps/notes/app.yaml:3 names the tile `library/tiles/bell`, which is not in the art library",
                "apps/notes/app.yaml:11 a schedule runs `morning`, but there is no workflows/morning.js",
                "apps/notes/stores/notes.js:2 opens the collection `notez`, which app.yaml does not declare",
            ]
        );
    }

    /// **A raised version needs each step's migration; a lowered one is
    /// refused; a new collection needs none.**
    #[test]
    fn a_version_rises_with_its_migrations() {
        let mut f = files(&[
            ("apps/notes/workflows/morning.js", ""),
            ("apps/notes/workflows/migrate-notes-2.js", ""),
        ]);
        f.name("library/tiles/bell.png");
        assert_eq!(
            run(&f, Some(&was(1))),
            ["apps/notes/app.yaml:5 raises notes to version 3 without workflows/migrate-notes-3.js, which turns a record at version 2 into one at 3"]
        );
        f.add("apps/notes/workflows/migrate-notes-3.js", Vec::new());
        assert_eq!(run(&f, Some(&was(1))), Vec::<String>::new());
        assert_eq!(run(&f, Some(&was(3))), Vec::<String>::new(), "unchanged");
        assert_eq!(
            run(&f, Some(&was(4))),
            ["apps/notes/app.yaml:5 lowers notes's version from 4 to 3; a version only rises"]
        );
        let mut other = was(9);
        other.collections = Default::default();
        assert_eq!(
            run(&f, Some(&other)),
            Vec::<String>::new(),
            "a new collection"
        );
    }
}
