//! The gate against real repositories: an origin, a writer pushing
//! candidates to it as Zen does, and the live checkout the gate fetches into.

use std::path::Path;
use std::process::Command;

use super::*;
use crate::check::tests::recipes;

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

fn put(root: &Path, path: &str, text: &str) {
    let p = root.join(path);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
}

struct Repos {
    _origin: tempfile::TempDir,
    writer: tempfile::TempDir,
    live: tempfile::TempDir,
    main: String,
}

/// An origin whose `main` holds the recipes app, a writer and the live clone.
fn repos() -> Repos {
    let origin = tempfile::tempdir().unwrap();
    run(origin.path(), &["init", "-q", "--bare", "-b", "main"]);
    let url = origin.path().to_string_lossy().to_string();
    let writer = tempfile::tempdir().unwrap();
    run(writer.path(), &["init", "-q", "-b", "main"]);
    run(writer.path(), &["config", "core.autocrlf", "false"]);
    for (path, text) in &recipes().text {
        put(writer.path(), path, text);
    }
    run(writer.path(), &["add", "."]);
    run(writer.path(), &["commit", "-q", "-m", "recipes"]);
    run(writer.path(), &["remote", "add", "origin", &url]);
    run(writer.path(), &["push", "-q", "origin", "main"]);
    let main = run(writer.path(), &["rev-parse", "HEAD"]);
    let live = tempfile::tempdir().unwrap();
    run(
        live.path(),
        &["clone", "-q", "-c", "core.autocrlf=false", &url, "."],
    );
    Repos {
        _origin: origin,
        writer,
        live,
        main,
    }
}

/// Commit `changes` on the writer's `main` and push it as candidate `id`,
/// leaving origin's `main` where it was: as Zen does before it asks.
fn candidate(r: &Repos, id: &str, changes: &[(&str, &str)]) -> Ask {
    for (path, text) in changes {
        put(r.writer.path(), path, text);
    }
    run(r.writer.path(), &["add", "."]);
    run(r.writer.path(), &["commit", "-q", "-m", id]);
    let commit = run(r.writer.path(), &["rev-parse", "HEAD"]);
    let reference = format!("refs/heads/zen/candidate/{id}");
    run(
        r.writer.path(),
        &["push", "-q", "origin", &format!("HEAD:{reference}")],
    );
    run(r.writer.path(), &["reset", "-q", "--hard", &r.main]);
    Ask {
        id: id.into(),
        reference: Some(reference),
        commit: Some(commit),
        base: Some(r.main.clone()),
        branch: "main".into(),
        delete: false,
    }
}

/// **A good change is let through, and the gate leaves no ref behind.**
#[test]
fn a_good_change_is_let_through() {
    let r = repos();
    let ask = candidate(
        &r,
        "c1",
        &[(
            "apps/recipes/style.css",
            "recipe-card { margin: var(--yen-space-3); }\n",
        )],
    );
    assert_eq!(answer(r.live.path(), &ask), Answer::Ok);
    assert_eq!(run(r.live.path(), &["for-each-ref", "refs/yenbot"]), "");
}

/// **A bad change is refused with its file, line and what to do instead.**
#[test]
fn a_bad_change_is_refused_with_where_and_why() {
    let r = repos();
    let ask = candidate(
        &r,
        "c2",
        &[
            (
                "apps/recipes/style.css",
                "recipe-card {\n  color: #c2375b;\n}\n",
            ),
            ("apps/recipes/notes.txt", "ideas"),
        ],
    );
    assert_eq!(
        answer(r.live.path(), &ask),
        Answer::Refused(
            "apps/recipes/notes.txt is outside an app's shape: app.yaml, index.html, app.js and style.css, and \
             JavaScript in pages/, components/, stores/ and workflows/\n\
             apps/recipes/style.css:2 uses the colour `#c2375b`; use a token: var(--yen-accent), var(--yen-ink), …"
                .into()
        )
    );
    assert_eq!(run(r.live.path(), &["for-each-ref", "refs/yenbot"]), "");
    // The live checkout's working tree is untouched.
    assert_eq!(
        std::fs::read_to_string(r.live.path().join("apps/recipes/style.css")).unwrap(),
        recipes().text["apps/recipes/style.css"]
    );
}

/// **Only the apps a change touches are checked, against what main held**:
/// a version raised without its migration is refused, and an app removed is
/// nothing to check.
#[test]
fn a_change_is_checked_against_what_main_held() {
    let r = repos();
    let raised = recipes().text["apps/recipes/app.yaml"]
        .replace("    fields:", "    version: 2\n    fields:");
    let ask = candidate(&r, "c3", &[("apps/recipes/app.yaml", &raised)]);
    let Answer::Refused(why) = answer(r.live.path(), &ask) else {
        panic!("let through")
    };
    assert_eq!(
        why,
        "apps/recipes/app.yaml:7 raises recipes to version 2 without workflows/migrate-recipes-2.js, which turns a \
         record at version 1 into one at 2"
    );
    run(r.writer.path(), &["rm", "-q", "-r", "apps/recipes"]);
    let gone = candidate(&r, "c4", &[]);
    assert_eq!(answer(r.live.path(), &gone), Answer::Ok);
}

/// **Another branch is answered at once; main is never deleted; a candidate
/// that is not the commit asked about, or cannot be read, is refused.**
#[test]
fn the_gate_refuses_what_it_cannot_vouch_for() {
    let r = repos();
    let mut other = candidate(&r, "c5", &[("apps/recipes/notes.txt", "x")]);
    other.branch = "zen/work".into();
    assert_eq!(answer(r.live.path(), &other), Answer::Ok);
    let delete = Ask {
        id: "c6".into(),
        reference: None,
        commit: None,
        base: Some(r.main.clone()),
        branch: "main".into(),
        delete: true,
    };
    assert_eq!(
        answer(r.live.path(), &delete),
        Answer::Refused("main is the branch the site serves, and is never deleted".into())
    );
    let mut wrong = candidate(
        &r,
        "c7",
        &[("apps/recipes/style.css", "a { margin: 0; }\n")],
    );
    wrong.commit = Some(r.main.clone());
    let Answer::Refused(why) = answer(r.live.path(), &wrong) else {
        panic!("let through")
    };
    assert!(
        why.starts_with("the gate could not read the candidate: the candidate holds "),
        "{why}"
    );
    let mut missing = wrong.clone();
    missing.reference = Some("refs/heads/zen/candidate/none".into());
    let Answer::Refused(why) = answer(r.live.path(), &missing) else {
        panic!("let through")
    };
    assert!(
        why.starts_with("the gate could not read the candidate: git fetch"),
        "{why}"
    );
}
