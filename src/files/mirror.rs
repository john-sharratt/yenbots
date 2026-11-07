//! The files beside the backups (§7.4): a file never changes once kept, so
//! the nightly backup copies each new one, once, into the backups' own
//! `files/`; restoring an app's records brings back any of its files that
//! is missing.

use std::path::Path;

use anyhow::{Context, Result};

/// Copy every file under `from` that `to` lacks; answers how many.
pub fn mirror(from: &Path, to: &Path) -> Result<u64> {
    let Ok(apps) = std::fs::read_dir(from) else {
        return Ok(0);
    };
    let mut copied = 0;
    for app in apps.flatten().filter(|e| e.path().is_dir()) {
        let into = to.join(app.file_name());
        std::fs::create_dir_all(&into).with_context(|| format!("making {}", into.display()))?;
        for f in std::fs::read_dir(app.path())?.flatten() {
            let name = f.file_name();
            if name.to_string_lossy().ends_with(".part") {
                continue;
            }
            let target = into.join(&name);
            if !target.exists() {
                let part = into.join(format!("{}.part", name.to_string_lossy()));
                std::fs::copy(f.path(), &part)
                    .with_context(|| format!("copying {}", f.path().display()))?;
                std::fs::rename(&part, &target)?;
                copied += 1;
            }
        }
    }
    Ok(copied)
}

/// Bring back app `app`'s files named `ids` that `files` lacks, from the
/// backups' `kept`; answers how many came back.
pub fn bring_back(kept: &Path, files: &Path, app: &str, ids: &[String]) -> Result<u64> {
    let mut back = 0;
    std::fs::create_dir_all(files.join(app))?;
    for id in ids {
        for name in [id.clone(), format!("{id}.thumb.webp")] {
            let (src, dst) = (kept.join(app).join(&name), files.join(app).join(&name));
            if !dst.exists() && src.exists() {
                std::fs::copy(&src, &dst)
                    .with_context(|| format!("restoring {}", dst.display()))?;
                back += 1;
            }
        }
    }
    Ok(back)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_files_are_copied_once_and_brought_back() {
        let dir = tempfile::tempdir().unwrap();
        let (files, kept) = (dir.path().join("files"), dir.path().join("backups/files"));
        std::fs::create_dir_all(files.join("recipes")).unwrap();
        std::fs::write(files.join("recipes/f1"), b"one").unwrap();
        std::fs::write(files.join("recipes/f1.thumb.webp"), b"t").unwrap();
        std::fs::write(files.join("recipes/f2.part"), b"cut short").unwrap();
        assert_eq!(mirror(&files, &kept).unwrap(), 2);
        assert_eq!(mirror(&files, &kept).unwrap(), 0, "once");
        assert_eq!(std::fs::read(kept.join("recipes/f1")).unwrap(), b"one");
        assert!(!kept.join("recipes/f2.part").exists());

        std::fs::remove_file(files.join("recipes/f1")).unwrap();
        std::fs::remove_file(files.join("recipes/f1.thumb.webp")).unwrap();
        let ids = ["f1".to_string(), "gone".to_string()];
        assert_eq!(bring_back(&kept, &files, "recipes", &ids).unwrap(), 2);
        assert_eq!(std::fs::read(files.join("recipes/f1")).unwrap(), b"one");
    }
}
