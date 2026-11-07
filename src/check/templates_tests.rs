//! The templates (`templates/`, §16) held to the check: each is a working
//! app a new one starts as a copy of, so each passes every rule as a new app
//! under its own id, with the library and the guide as the repository holds
//! them.

use std::collections::BTreeSet;
use std::path::Path;

use super::files::{from_disk, Files};
use super::{check_app, told, Before};
use crate::apps::manifest::Manifest;

/// The templates the design names, by id (§16).
const TEMPLATES: [&str; 11] = [
    "assistant",
    "blank",
    "collection",
    "daily-digest",
    "form",
    "journal",
    "list",
    "picture-maker",
    "planner",
    "reminders",
    "tracker",
];

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// Template `id`'s files as an app's — `apps/<id>/…` — with the library
/// and the guide.
fn as_app(id: &str) -> Files {
    let mut files = from_disk(root(), "").unwrap();
    files.names.retain(|n| !n.starts_with("apps/"));
    files.text.retain(|n, _| !n.starts_with("apps/"));
    let dir = root().join("templates").join(id);
    add_under(&dir, &dir, id, &mut files);
    files
}

fn add_under(base: &Path, dir: &Path, id: &str, files: &mut Files) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            add_under(base, &path, id, files);
            continue;
        }
        let rel = path
            .strip_prefix(base)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        files.add(&format!("apps/{id}/{rel}"), std::fs::read(&path).unwrap());
    }
}

/// **The templates are the eleven the design names**, one folder each.
#[test]
fn the_templates_are_the_designs() {
    let found: BTreeSet<String> = std::fs::read_dir(root().join("templates"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_dir())
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    assert_eq!(found, TEMPLATES.iter().map(|t| t.to_string()).collect());
}

/// **Every template passes the check as a new app**, built to the guide's
/// current edition, and is a draft until Yen says otherwise.
#[test]
fn every_template_passes_the_check() {
    let edition = from_disk(root(), "").unwrap().guide_edition();
    assert!(edition.is_some(), "the guide states its edition");
    for id in TEMPLATES {
        let files = as_app(id);
        let refusals = check_app(&files, id, &Before { manifest: None });
        assert!(refusals.is_empty(), "{id}:\n{}", told(&refusals));
        let text = &files.text[&format!("apps/{id}/app.yaml")];
        let m = Manifest::parse(text, id).unwrap();
        assert_eq!(m.guide, edition, "{id} is built to the current guide");
        assert!(m.tile.is_some(), "{id} has a tile");
        assert!(text.contains("status: draft"), "{id} starts as a draft");
    }
}
