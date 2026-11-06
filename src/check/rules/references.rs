//! `references`: everything an app names is there — its tile and every
//! picture its pages name in the library's catalogue, each schedule's
//! workflow, each collection its code opens — and a collection's version
//! only rises, each step with the migration that takes a record up it
//! (§7.5).

use regex::Regex;

use crate::apps::manifest::Manifest;
use crate::check::files::Files;
use crate::check::source::{css_without_comments, line_of};
use crate::check::{Before, Refusal};
use crate::library::check::keys;

use super::{app_dir, scripts};

pub const RULE: &str = "references";

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

    let library = keys(files);
    if let Some(tile) = &m.tile {
        let there = tile
            .strip_prefix("library/")
            .is_some_and(|key| key.starts_with("tiles/") && library.contains(key));
        if !there {
            refuse(
                out,
                "tile:",
                format!("names the tile `{tile}`, which is not in the art library (/_library?kind=tiles)"),
            );
        }
    }

    // `<yen-empty art="no-recipes">` names `empty/no-recipes`; a URL names
    // `/_library/<kind>/<id>-<width>.webp`.
    let art = Regex::new(r#"\bart="([^"$]+)""#).unwrap();
    let url = Regex::new(r"/_library/([a-z]+/[a-z0-9-]+)-[0-9]+\.webp").unwrap();
    let mut texts = scripts(files, id);
    for (path, text) in files.texts_under(&dir) {
        if path.ends_with(".css") {
            texts.push((path.to_string(), css_without_comments(text)));
        } else if path.ends_with(".html") {
            texts.push((path.to_string(), text.to_string()));
        }
    }
    for (path, text) in texts {
        let named = art
            .captures_iter(&text)
            .map(|c| (c.get(1).unwrap().start(), format!("empty/{}", &c[1])))
            .chain(
                url.captures_iter(&text)
                    .map(|c| (c.get(1).unwrap().start(), c[1].to_string())),
            );
        for (at, key) in named {
            if !library.contains(&key) {
                out.push(Refusal::new(
                    RULE,
                    path.clone(),
                    Some(line_of(&text, at)),
                    format!(
                        "names the picture `{key}`, which is not in the art library (/_library)"
                    ),
                ));
            }
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
    use crate::library::catalogue::PATH;

    const YAML: &str = "id: notes\nname: Notes\ntile: library/tiles/bell\ncollections:\n  notes:\n    version: 3\n    fields:\n      text: { type: text }\nschedules:\n  - every: daily 07:30\n    run: morning\n";

    /// The library: a bell tile and one empty state.
    const CATALOGUE: &str = "tiles/bell:\n  tags: [bell]\n  seed: 1\n  prompt: A bell.\nempty/no-notes:\n  tags: [notes]\n  seed: 2\n  prompt: A blank page.\n";

    fn with_library(f: &mut Files) {
        f.add(PATH, CATALOGUE.as_bytes().to_vec());
    }

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
        out.sort();
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
            (
                "apps/notes/pages/home.js",
                "html`<yen-empty art=\"no-notes\" heading=\"No notes\"></yen-empty><yen-empty art=${x}></yen-empty>`",
            ),
            (
                "apps/notes/style.css",
                "header { background: url(\"/_library/tiles/bell-256.webp\"); }",
            ),
        ]);
        with_library(&mut f);
        assert_eq!(run(&f, None), Vec::<String>::new());
    }

    #[test]
    fn what_is_not_there_is_refused_on_its_line() {
        let f = files(&[
            (
                "apps/notes/stores/notes.js",
                "// app.collection(\"commented\")\nexport const notes = app.collection('notez');",
            ),
            (
                "apps/notes/pages/home.js",
                "// art=\"commented\"\nhtml`<yen-empty art=\"no-recipes\"></yen-empty>`",
            ),
            (
                "apps/notes/index.html",
                "<link rel=\"icon\" href=\"/_library/tiles/cup-128.webp\">",
            ),
        ]);
        assert_eq!(
            run(&f, None),
            [
                "apps/notes/app.yaml:3 names the tile `library/tiles/bell`, which is not in the art library (/_library?kind=tiles)",
                "apps/notes/app.yaml:11 a schedule runs `morning`, but there is no workflows/morning.js",
                "apps/notes/index.html:1 names the picture `tiles/cup`, which is not in the art library (/_library)",
                "apps/notes/pages/home.js:2 names the picture `empty/no-recipes`, which is not in the art library (/_library)",
                "apps/notes/stores/notes.js:2 opens the collection `notez`, which app.yaml does not declare",
            ]
        );
    }

    /// **A tile is a tile**: a picture of another kind is refused as one.
    #[test]
    fn a_tile_names_a_tile() {
        let mut f = files(&[("apps/notes/workflows/morning.js", "")]);
        with_library(&mut f);
        let yaml = YAML.replace("library/tiles/bell", "library/empty/no-notes");
        f.add("apps/notes/app.yaml", yaml.into_bytes());
        let m = Manifest::parse(f.text["apps/notes/app.yaml"].as_str(), "notes").unwrap();
        let mut out = Vec::new();
        check(&f, "notes", &m, &Before { manifest: None }, &mut out);
        assert_eq!(
            out.iter().map(Refusal::to_string).collect::<Vec<_>>(),
            ["apps/notes/app.yaml:3 names the tile `library/empty/no-notes`, which is not in the art library (/_library?kind=tiles)"]
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
        with_library(&mut f);
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
