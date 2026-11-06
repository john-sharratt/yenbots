//! Running `git` on a repository the site holds — the live checkout — for
//! the publisher, the gate and the check's reader. A command that fails is
//! an error carrying git's own words.

use std::path::Path;
use std::process::Command;

use anyhow::{bail, Result};

/// `git -C <repo> <args>`'s output, as bytes.
pub fn git_bytes(repo: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()?;
    if !out.status.success() {
        bail!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(out.stdout)
}

/// `git -C <repo> <args>`'s output, as trimmed text.
pub fn git(repo: &Path, args: &[&str]) -> Result<String> {
    Ok(String::from_utf8_lossy(&git_bytes(repo, args)?)
        .trim()
        .to_string())
}
