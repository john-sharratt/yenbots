//! The live checkout (`candle/docs/yenbot_app_host.md` §4, §5.4): a clone of
//! the repository that follows `origin/main` and nothing else. It is moved
//! only forward — fetched, then fast-forwarded — so a history rewrite on
//! origin is refused and logged, and the site keeps serving what it had.

use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, bail, Result};

/// What a fast-forward did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Moved {
    /// Already at `origin/main`.
    Unchanged,
    /// Moved from one commit to the other.
    To { from: String, to: String },
}

fn git(dir: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git").arg("-C").arg(dir).args(args).output()?;
    if !out.status.success() {
        bail!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Fetch `origin/main` into `live` and fast-forward to it.
pub fn fast_forward(live: &Path) -> Result<Moved> {
    if !live.join(".git").exists() {
        return Err(anyhow!(
            "{} is not a checkout: clone the repository there first (docs/running.md)",
            live.display()
        ));
    }
    // The site serves these files as committed, byte for byte: a checkout
    // that converts line endings serves something no commit holds, every
    // file it has already checked out included, so it is refused whole.
    let crlf = git(live, &["config", "--get", "core.autocrlf"]).unwrap_or_default();
    if crlf != "false" {
        bail!(
            "{} converts line endings (core.autocrlf={}): clone it again with `git clone -c \
             core.autocrlf=false` (docs/running.md)",
            live.display(),
            if crlf.is_empty() { "unset" } else { &crlf }
        );
    }
    let from = git(live, &["rev-parse", "HEAD"])?;
    git(live, &["fetch", "-q", "origin", "main"])?;
    let target = git(live, &["rev-parse", "origin/main"])?;
    if target == from {
        return Ok(Moved::Unchanged);
    }
    if git(live, &["merge-base", "--is-ancestor", &from, &target]).is_err() {
        bail!(
            "origin/main ({target}) does not descend from what the site serves ({from}): a \
             history rewrite is refused, and the site keeps serving {from}"
        );
    }
    git(live, &["merge", "-q", "--ff-only", "origin/main"])?;
    Ok(Moved::To { from, to: target })
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// An origin with one commit, a writer's clone of it, and the live clone.
    fn setup() -> (tempfile::TempDir, tempfile::TempDir, tempfile::TempDir) {
        let origin = tempfile::tempdir().unwrap();
        run(origin.path(), &["init", "-q", "--bare", "-b", "main"]);
        let url = origin.path().to_string_lossy().to_string();
        let writer = tempfile::tempdir().unwrap();
        run(writer.path(), &["init", "-q", "-b", "main"]);
        std::fs::write(writer.path().join("a.txt"), "one\n").unwrap();
        run(writer.path(), &["add", "a.txt"]);
        run(writer.path(), &["commit", "-q", "-m", "one"]);
        run(writer.path(), &["remote", "add", "origin", &url]);
        run(writer.path(), &["push", "-q", "origin", "main"]);
        let live = tempfile::tempdir().unwrap();
        run(
            live.path(),
            &["clone", "-q", "-c", "core.autocrlf=false", &url, "."],
        );
        (origin, writer, live)
    }

    /// **A push lands in the live checkout**, and a second look moves nothing.
    #[test]
    fn a_push_is_fast_forwarded_into_the_live_checkout() {
        let (_origin, writer, live) = setup();
        assert_eq!(fast_forward(live.path()).unwrap(), Moved::Unchanged);
        let from = run(live.path(), &["rev-parse", "HEAD"]);
        std::fs::write(writer.path().join("a.txt"), "two\n").unwrap();
        run(writer.path(), &["commit", "-q", "-am", "two"]);
        run(writer.path(), &["push", "-q", "origin", "main"]);
        let to = run(writer.path(), &["rev-parse", "HEAD"]);
        assert_eq!(
            fast_forward(live.path()).unwrap(),
            Moved::To {
                from,
                to: to.clone()
            }
        );
        assert_eq!(
            std::fs::read_to_string(live.path().join("a.txt")).unwrap(),
            "two\n"
        );
        assert_eq!(fast_forward(live.path()).unwrap(), Moved::Unchanged);
    }

    /// **A rewritten history is refused**, and the site keeps what it had.
    #[test]
    fn a_rewritten_history_is_refused() {
        let (_origin, writer, live) = setup();
        std::fs::write(writer.path().join("a.txt"), "two\n").unwrap();
        run(writer.path(), &["commit", "-q", "-am", "two"]);
        run(writer.path(), &["push", "-q", "origin", "main"]);
        fast_forward(live.path()).unwrap();
        run(writer.path(), &["reset", "-q", "--hard", "HEAD~1"]);
        std::fs::write(writer.path().join("a.txt"), "other\n").unwrap();
        run(writer.path(), &["commit", "-q", "-am", "other"]);
        run(writer.path(), &["push", "-q", "--force", "origin", "main"]);
        let err = fast_forward(live.path()).unwrap_err().to_string();
        assert!(err.contains("history rewrite is refused"), "{err}");
        assert_eq!(
            std::fs::read_to_string(live.path().join("a.txt")).unwrap(),
            "two\n"
        );
    }

    /// **A checkout that converts line endings is refused**, with how to
    /// clone it again.
    #[test]
    fn a_checkout_converting_line_endings_is_refused() {
        let (_origin, _writer, live) = setup();
        run(live.path(), &["config", "core.autocrlf", "true"]);
        let err = fast_forward(live.path()).unwrap_err().to_string();
        assert!(
            err.contains("converts line endings (core.autocrlf=true)"),
            "{err}"
        );
    }

    #[test]
    fn a_folder_that_is_no_checkout_is_said_so() {
        let dir = tempfile::tempdir().unwrap();
        assert!(fast_forward(dir.path())
            .unwrap_err()
            .to_string()
            .contains("is not a checkout"));
    }
}
