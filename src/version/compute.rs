//! A build's version, from its git checkout — the rule the Tokera cluster's
//! binaries follow, so keep judges this one the same way. Compiled twice: by
//! `build.rs`, which stamps the result into the binary, and by the tests.
//!
//! - `count`: commits on `origin/main` that HEAD contains — the commit count of
//!   `git merge-base HEAD origin/main`. A local commit does not move it.
//! - `hash`: that merge base, short.
//! - `local`: empty when the build's inputs ([`INPUTS`]) are exactly the merge
//!   base's; otherwise the first 16 hex digits of a SHA-256 over how they
//!   differ — the diff against the merge base, and every untracked input file,
//!   by path and contents. keep tries only a build whose `local` is empty.
//!
//! The apps, the art library and the docs are not inputs: the binary reads
//! them from disk at runtime, so changing them is never a new build.

use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

/// What the binary is built from, as git pathspecs from the repository root.
pub const INPUTS: [&str; 5] = ["Cargo.toml", "Cargo.lock", "build.rs", "src", "web"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    pub count: u32,
    pub hash: String,
    pub local: String,
}

fn git(root: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .ok()?;
    out.status.success().then_some(out.stdout)
}

fn git_text(root: &Path, args: &[&str]) -> Option<String> {
    Some(
        String::from_utf8_lossy(&git(root, args)?)
            .trim()
            .to_string(),
    )
}

pub fn compute(root: &Path) -> Version {
    let base = git_text(root, &["merge-base", "HEAD", "origin/main"]);
    let (count, hash) = match &base {
        Some(base) => (
            git_text(root, &["rev-list", "--count", base])
                .and_then(|c| c.parse().ok())
                .unwrap_or(0),
            git_text(root, &["rev-parse", "--short", base]).unwrap_or_else(|| "none".into()),
        ),
        None => (0, "none".into()),
    };
    let scoped = |args: &[&str]| -> Option<Vec<u8>> {
        let mut all: Vec<&str> = args.to_vec();
        all.push("--");
        all.extend(INPUTS);
        git(root, &all)
    };
    let mut digest = Sha256::new();
    let mut beyond = false;
    let against = match &base {
        Some(base) => base.clone(),
        None => {
            beyond = true;
            digest.update(b"commits\0");
            digest.update(git(root, &["rev-list", "HEAD"]).unwrap_or_else(|| b"no git".to_vec()));
            "HEAD".to_string()
        }
    };
    let diff = scoped(&["diff", &against, "--binary"]).unwrap_or_default();
    if !diff.is_empty() {
        beyond = true;
        digest.update(b"diff\0");
        digest.update(&diff);
    }
    let untracked =
        scoped(&["ls-files", "--others", "--exclude-standard", "-z"]).unwrap_or_default();
    for path in untracked.split(|b| *b == 0).filter(|p| !p.is_empty()) {
        beyond = true;
        digest.update(b"untracked\0");
        digest.update(path);
        digest.update(b"\0");
        let file = root.join(String::from_utf8_lossy(path).as_ref());
        digest.update(std::fs::read(file).unwrap_or_default());
    }
    let local = if beyond {
        digest
            .finalize()
            .iter()
            .take(8)
            .map(|b| format!("{b:02x}"))
            .collect()
    } else {
        String::new()
    };
    Version { count, hash, local }
}

/// What the build script reruns on: git's HEAD, the refs it and `origin/main`
/// name, the packed refs, and every input.
pub fn rerun_paths(root: &Path) -> Vec<PathBuf> {
    let git_dir = git_text(root, &["rev-parse", "--git-dir"])
        .map(|p| {
            let p = PathBuf::from(p);
            if p.is_absolute() {
                p
            } else {
                root.join(p)
            }
        })
        .unwrap_or_else(|| root.join(".git"));
    let mut paths = vec![
        git_dir.join("HEAD"),
        git_dir.join("packed-refs"),
        git_dir
            .join("refs")
            .join("remotes")
            .join("origin")
            .join("main"),
    ];
    if let Ok(head) = std::fs::read_to_string(git_dir.join("HEAD")) {
        if let Some(r) = head.trim().strip_prefix("ref: ") {
            paths.push(git_dir.join(r));
        }
    }
    paths.extend(INPUTS.iter().map(|i| root.join(i)));
    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(dir: &Path, args: &[&str]) {
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
    }

    /// A checkout of a repository with one commit on `origin/main`.
    fn checkout() -> (tempfile::TempDir, tempfile::TempDir) {
        let origin = tempfile::tempdir().unwrap();
        run(origin.path(), &["init", "-q", "--bare", "-b", "main"]);
        let work = tempfile::tempdir().unwrap();
        run(work.path(), &["init", "-q", "-b", "main"]);
        std::fs::create_dir_all(work.path().join("src")).unwrap();
        std::fs::create_dir_all(work.path().join("docs")).unwrap();
        std::fs::write(work.path().join("Cargo.toml"), "[package]\n").unwrap();
        std::fs::write(work.path().join("src/main.rs"), "fn main() {}\n").unwrap();
        std::fs::write(work.path().join("docs/index.md"), "# Docs\n").unwrap();
        run(work.path(), &["add", "-A"]);
        run(work.path(), &["commit", "-q", "-m", "first"]);
        let url = origin.path().to_string_lossy().to_string();
        run(work.path(), &["remote", "add", "origin", &url]);
        run(work.path(), &["push", "-q", "origin", "main"]);
        run(work.path(), &["fetch", "-q", "origin"]);
        (origin, work)
    }

    /// **A clean build of a commit on `origin/main` has no local part**, and
    /// a change to a doc — not an input — leaves it clean.
    #[test]
    fn a_clean_build_is_not_local_and_docs_are_not_inputs() {
        let (_origin, work) = checkout();
        let v = compute(work.path());
        assert_eq!((v.count, v.local.as_str()), (1, ""));
        assert_eq!(v.hash.len(), 7);
        std::fs::write(work.path().join("docs/index.md"), "# Changed\n").unwrap();
        std::fs::write(work.path().join("docs/new.md"), "new\n").unwrap();
        assert_eq!(compute(work.path()).local, "");
    }

    /// **The build reruns on git's HEAD, the branch it names, and every
    /// input** — never the apps or the docs.
    #[test]
    fn the_build_reruns_on_head_and_the_inputs() {
        let (_origin, work) = checkout();
        let paths = rerun_paths(work.path());
        let git = work.path().join(".git");
        assert!(paths.contains(&git.join("HEAD")));
        assert!(paths.contains(&git.join("refs/heads/main")));
        for input in INPUTS {
            assert!(paths.contains(&work.path().join(input)), "{input}");
        }
        assert!(!paths.contains(&work.path().join("docs")));
    }

    /// **A changed or new input makes the build local**, named by its
    /// content: the same change names the same build.
    #[test]
    fn a_changed_input_makes_the_build_local() {
        let (_origin, work) = checkout();
        std::fs::write(work.path().join("src/main.rs"), "fn main() { }\n").unwrap();
        let changed = compute(work.path()).local;
        assert_eq!(changed.len(), 16);
        assert_eq!(compute(work.path()).local, changed);
        std::fs::write(work.path().join("src/extra.rs"), "\n").unwrap();
        let more = compute(work.path()).local;
        assert_eq!(more.len(), 16);
        assert_ne!(more, changed);
    }
}
