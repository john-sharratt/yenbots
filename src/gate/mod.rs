//! The gate (`candle/docs/yenbot_app_host.md` §5.4, §17.4): before Zen moves
//! `main`, it asks on the bus whether the candidate may land, and yenbot
//! answers. The candidate is fetched into a ref of the gate's own in the
//! live checkout — never into its working tree — and the check runs on every
//! app the candidate changes, against what `main` held. A candidate that
//! cannot be read is refused: nothing lands unchecked.
//!
//! Only `main` is served, so a push to any other branch is answered at once,
//! and `main` itself is never deleted.

pub mod ask;

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{bail, Result};

use crate::apps::manifest::Manifest;
use crate::check::files::from_commit;
use crate::check::{check_app, told, Before, Refusal};
use crate::git::git;
use crate::publish::BRANCH;
use ask::{Answer, Ask};

/// The apps a change from `base` to `commit` touches: every `apps/<id>/`
/// with a path that differs — every app, for a branch with no base.
fn touched(repo: &Path, base: Option<&str>, commit: &str) -> Result<BTreeSet<String>> {
    let paths = match base {
        Some(base) => git(repo, &["diff", "--name-only", base, commit, "--", "apps"])?,
        None => git(
            repo,
            &["ls-tree", "-r", "--name-only", commit, "--", "apps"],
        )?,
    };
    Ok(paths
        .lines()
        .filter_map(|p| {
            p.strip_prefix("apps/")?
                .split_once('/')
                .map(|(id, _)| id.to_string())
        })
        .collect())
}

/// App `id`'s manifest as `commit` holds it, when it holds one the platform
/// takes.
fn manifest_at(repo: &Path, commit: &str, id: &str) -> Option<Manifest> {
    let text = git(repo, &["show", &format!("{commit}:apps/{id}/app.yaml")]).ok()?;
    Manifest::parse(&text, id).ok()
}

/// Every refusal of the apps `commit` changes from `base`.
pub fn check_commit(repo: &Path, base: Option<&str>, commit: &str) -> Result<Vec<Refusal>> {
    let mut refused = Vec::new();
    for id in touched(repo, base, commit)? {
        let files = from_commit(repo, commit, &id)?;
        if files.names_under(&format!("apps/{id}/")).next().is_none() {
            // Removed: nothing left to check.
            continue;
        }
        let was = base.and_then(|b| manifest_at(repo, b, &id));
        refused.extend(check_app(
            &files,
            &id,
            &Before {
                manifest: was.as_ref(),
            },
        ));
    }
    refused.sort();
    Ok(refused)
}

/// The answer to `ask`, with the candidate fetched into the repository at
/// `repo`.
pub fn answer(repo: &Path, ask: &Ask) -> Answer {
    if ask.delete {
        return if ask.branch == BRANCH {
            Answer::Refused(format!(
                "{BRANCH} is the branch the site serves, and is never deleted"
            ))
        } else {
            Answer::Ok
        };
    }
    if ask.branch != BRANCH {
        return Answer::Ok;
    }
    let (Some(reference), Some(commit)) = (&ask.reference, &ask.commit) else {
        return Answer::Refused("a move of main names its candidate and its commit".into());
    };
    if !ask.id_is_plain() {
        return Answer::Refused(format!("`{}` is not a candidate's id", ask.id));
    }
    let held = format!("refs/yenbot/gate/{}", ask.id);
    let result = fetch_and_check(repo, reference, &held, commit, ask.base.as_deref());
    // The gate's ref goes whatever the answer.
    let _ = git(repo, &["update-ref", "-d", &held]);
    match result {
        Ok(refused) if refused.is_empty() => Answer::Ok,
        Ok(refused) => Answer::Refused(told(&refused)),
        Err(e) => Answer::Refused(format!("the gate could not read the candidate: {e:#}")),
    }
}

fn fetch_and_check(
    repo: &Path,
    reference: &str,
    held: &str,
    commit: &str,
    base: Option<&str>,
) -> Result<Vec<Refusal>> {
    git(
        repo,
        &[
            "fetch",
            "-q",
            "--no-write-fetch-head",
            "origin",
            &format!("+{reference}:{held}"),
        ],
    )?;
    let got = git(repo, &["rev-parse", held])?;
    if got != commit {
        bail!("the candidate holds {got}, not {commit}");
    }
    check_commit(repo, base, commit)
}

#[cfg(test)]
mod tests;
