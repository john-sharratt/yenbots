//! An app's workflow files as it is served (§5.3): from the live checkout's
//! working tree, or — while what is there is refused — from the commit it
//! last passed at, so a run is always of the version the app is served at.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};

use super::loader::Sources;
use crate::apps::{Apps, FilesAt};
use crate::check::files::from_commit;

/// `apps/<id>/workflows/`.
pub fn dir_of(id: &str) -> String {
    format!("apps/{id}/workflows/")
}

/// The `.js` files under `dir` on disk, keyed `prefix` + their path below it.
fn walk(dir: &Path, prefix: &str, out: &mut BTreeMap<String, String>) -> Result<()> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(());
    };
    for e in entries {
        let e = e?;
        let name = e.file_name().to_string_lossy().to_string();
        let path = e.path();
        if path.is_dir() {
            walk(&path, &format!("{prefix}{name}/"), out)?;
        } else if name.ends_with(".js") {
            let text = std::fs::read_to_string(&path)
                .with_context(|| format!("reading {}", path.display()))?;
            out.insert(format!("{prefix}{name}"), text);
        }
    }
    Ok(())
}

/// App `id`'s workflow files, as it is served.
pub fn of(apps: &Apps, id: &str) -> Result<Sources> {
    let Some(FilesAt::Commit(commit)) = apps.files_at(id) else {
        return in_tree(apps.root(), id);
    };
    let dir = dir_of(id);
    let held = from_commit(apps.root(), &commit, id)?;
    let files = held
        .texts_under(&dir)
        .filter(|(path, _)| path.ends_with(".js"))
        .map(|(path, text)| (path.to_string(), text.to_string()))
        .collect();
    Ok(Sources {
        dir,
        files: Arc::new(files),
    })
}

/// App `id`'s workflow files in the working tree, whatever version it is
/// served at — those a migration to the version there runs.
pub fn in_tree(root: &Path, id: &str) -> Result<Sources> {
    let dir = dir_of(id);
    let mut files = BTreeMap::new();
    walk(
        &root.join("apps").join(id).join("workflows"),
        &dir,
        &mut files,
    )?;
    Ok(Sources {
        dir,
        files: Arc::new(files),
    })
}
