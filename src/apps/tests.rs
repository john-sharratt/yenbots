//! The apps as the live checkout holds them: read, checked, and kept at
//! their last good version when what arrives is refused.

use std::process::Command;

use super::*;

fn write(dir: &Path, app: &str, file: &str, text: &str) {
    let p = dir.join(app).join(file);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
}

fn run(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// **An app refused on reload keeps its last good version**; one that
/// passes again is served as it now is; one removed is gone.
#[test]
fn a_refused_app_keeps_its_last_good_version() {
    let live = tempfile::tempdir().unwrap();
    let dir = live.path().join("apps");
    write(&dir, "notes", "app.yaml", "id: notes\nname: Notes\n");
    std::fs::create_dir_all(dir.join("not-an-app")).unwrap();
    let apps = Apps::open(live.path());
    assert_eq!(apps.all().len(), 1);
    write(&dir, "notes", "app.yaml", "id: notes\nname: [broken\n");
    apps.reload();
    assert_eq!(apps.get("notes").unwrap().name, "Notes");
    assert_eq!(apps.checked("notes").unwrap().refusals.len(), 1);
    write(&dir, "notes", "app.yaml", "id: notes\nname: Notes Two\n");
    apps.reload();
    assert_eq!(apps.get("notes").unwrap().name, "Notes Two");
    assert_eq!(apps.checked("notes").unwrap().refusals, []);
    std::fs::remove_dir_all(dir.join("notes")).unwrap();
    apps.reload();
    assert!(apps.get("notes").is_none(), "a removed app is gone");
    assert!(apps.checked("notes").is_none());
}

/// **A change that arrived without the gate and fails the check is not
/// served: the app's files come from the commit it last passed at**, and
/// the check says why.
#[test]
fn a_refused_change_is_served_from_the_last_good_commit() {
    let live = tempfile::tempdir().unwrap();
    let dir = live.path().join("apps");
    write(&dir, "notes", "app.yaml", "id: notes\nname: Notes\n");
    write(
        &dir,
        "notes",
        "style.css",
        "a { margin: var(--yen-space-2); }\n",
    );
    run(live.path(), &["init", "-q", "-b", "main"]);
    run(live.path(), &["add", "."]);
    run(live.path(), &["commit", "-q", "-m", "good"]);
    let good = run(live.path(), &["rev-parse", "HEAD"]);
    let apps = Apps::open(live.path());
    assert_eq!(apps.files_at("notes"), Some(FilesAt::Tree));

    write(&dir, "notes", "style.css", "a {\n  color: red;\n}\n");
    run(live.path(), &["commit", "-q", "-am", "bad"]);
    let bad = run(live.path(), &["rev-parse", "HEAD"]);
    apps.reload();
    assert_eq!(apps.files_at("notes"), Some(FilesAt::Commit(good.clone())));
    let checked = apps.checked("notes").unwrap();
    assert_eq!(checked.commit.as_deref(), Some(bad.as_str()));
    assert_eq!(checked.serving.as_deref(), Some(good.as_str()));
    assert_eq!(
        checked.refusals.iter().map(Refusal::to_string).collect::<Vec<_>>(),
        ["apps/notes/style.css:2 uses the colour `red`; use a token: var(--yen-accent), var(--yen-ink), …"]
    );

    write(
        &dir,
        "notes",
        "style.css",
        "a { color: var(--yen-accent); }\n",
    );
    run(live.path(), &["commit", "-q", "-am", "fixed"]);
    apps.reload();
    assert_eq!(apps.files_at("notes"), Some(FilesAt::Tree));
    assert_eq!(apps.checked("notes").unwrap().serving, None);
}

/// **An app refused the first time it is seen is not served at all**, and
/// its check still says why.
#[test]
fn an_app_refused_at_first_sight_is_not_served() {
    let live = tempfile::tempdir().unwrap();
    let dir = live.path().join("apps");
    write(&dir, "notes", "app.yaml", "id: notes\nname: Notes\n");
    write(&dir, "notes", "helpers.js", "");
    let apps = Apps::open(live.path());
    assert!(apps.get("notes").is_none());
    assert_eq!(apps.files_at("notes"), None);
    let checked = apps.checked("notes").unwrap();
    assert_eq!(checked.serving, None);
    assert_eq!(checked.refusals[0].rule, "layout");
}
