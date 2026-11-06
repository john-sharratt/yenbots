//! The apps (`candle/docs/yenbot_app_host.md` §5): one folder each under the
//! live checkout's `apps/`, read at start and again each time the checkout
//! moves. An app whose manifest is refused keeps its last good version, and
//! the refusal is logged under the app's source.

pub mod access;
pub mod manifest;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use manifest::Manifest;

#[derive(Clone, Default)]
pub struct Apps {
    dir: PathBuf,
    loaded: Arc<RwLock<BTreeMap<String, Arc<Manifest>>>>,
}

impl Apps {
    /// The apps under `dir`, read now.
    pub fn open(dir: impl Into<PathBuf>) -> Apps {
        let apps = Apps {
            dir: dir.into(),
            loaded: Arc::default(),
        };
        apps.reload();
        apps
    }

    /// Read every app again; one refused keeps what it had.
    pub fn reload(&self) {
        let read = read_all(&self.dir);
        let mut loaded = self.loaded.write().unwrap_or_else(|e| e.into_inner());
        let mut next = BTreeMap::new();
        for (id, result) in read {
            match result {
                Ok(m) => {
                    next.insert(id, Arc::new(m));
                }
                Err(why) => {
                    tracing::warn!(target: "yenbot::apps", app = %id, "app.yaml refused: {why:#}");
                    if let Some(kept) = loaded.get(&id) {
                        next.insert(id, kept.clone());
                    }
                }
            }
        }
        *loaded = next;
    }

    pub fn get(&self, id: &str) -> Option<Arc<Manifest>> {
        self.loaded
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(id)
            .cloned()
    }

    pub fn all(&self) -> Vec<Arc<Manifest>> {
        self.loaded
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .cloned()
            .collect()
    }

    /// Apps held in memory, for tests.
    #[cfg(test)]
    pub fn of(manifests: Vec<Manifest>) -> Apps {
        let apps = Apps::default();
        *apps.loaded.write().unwrap() = manifests
            .into_iter()
            .map(|m| (m.id.clone(), Arc::new(m)))
            .collect();
        apps
    }
}

/// Each app folder under `dir` and its manifest, or why it was refused.
fn read_all(dir: &Path) -> Vec<(String, anyhow::Result<Manifest>)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<(String, anyhow::Result<Manifest>)> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let folder = e.file_name().to_str()?.to_string();
            let file = e.path().join("app.yaml");
            let text = std::fs::read_to_string(&file).ok()?;
            Some((folder.clone(), Manifest::parse(&text, &folder)))
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, app: &str, text: &str) {
        std::fs::create_dir_all(dir.join(app)).unwrap();
        std::fs::write(dir.join(app).join("app.yaml"), text).unwrap();
    }

    /// **An app refused on reload keeps its last good version.**
    #[test]
    fn a_refused_app_keeps_its_last_good_version() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "notes", "id: notes\nname: Notes\n");
        std::fs::create_dir_all(dir.path().join("not-an-app")).unwrap();
        let apps = Apps::open(dir.path());
        assert_eq!(apps.all().len(), 1);
        write(dir.path(), "notes", "id: notes\nname: [broken\n");
        apps.reload();
        assert_eq!(apps.get("notes").unwrap().name, "Notes");
        write(dir.path(), "notes", "id: notes\nname: Notes Two\n");
        apps.reload();
        assert_eq!(apps.get("notes").unwrap().name, "Notes Two");
        std::fs::remove_dir_all(dir.path().join("notes")).unwrap();
        apps.reload();
        assert!(apps.get("notes").is_none(), "a removed app is gone");
    }
}
