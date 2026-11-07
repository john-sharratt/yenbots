//! The apps (`candle/docs/yenbot_app_host.md` §5): one folder each under the
//! live checkout's `apps/`, read and checked ([`crate::check`]) at start and
//! again each time the checkout moves.
//!
//! The gate checks every change Zen makes before it lands; this check is
//! for anything that arrived without one — a push from a laptop — and an app
//! it refuses keeps its last good version: the manifest it had and the
//! files of the commit it last passed at, served from the repository's
//! objects rather than the working tree. The refusals are logged under the
//! app's name and kept for the admin page.
//!
//! An app that passes but raises a collection's version keeps its last good
//! version the same way while its records are migrated (§7.5,
//! [`crate::reshape`]); it is served as it now is once none is left behind.

pub mod access;
pub mod manifest;
pub mod schedule;
pub mod theme;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use serde::Serialize;
use tokio::sync::Notify;

use crate::check::files::from_disk;
use crate::check::{check_app, told, Before, Refusal};
use crate::git::git;
use crate::lifecycle::archive::ids;
use crate::reshape::migrate::behind;
use crate::store::Store;
use manifest::Manifest;

/// Where an app's files are served from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilesAt {
    /// The live checkout's working tree: the app as it is now.
    Tree,
    /// The commit the app last passed the check at, while what is now in
    /// the working tree is refused.
    Commit(String),
}

/// An app being served.
#[derive(Debug, Clone)]
struct Served {
    manifest: Arc<Manifest>,
    /// The checkout's commit when the app last passed; `None` in a folder
    /// that is no checkout.
    good: Option<String>,
    /// Whether what is now in the working tree is refused.
    kept: bool,
}

/// The last check of an app.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Checked {
    /// The checkout's commit checked.
    pub commit: Option<String>,
    pub refusals: Vec<Refusal>,
    /// The commit whose files are served while these are refused, or while
    /// its records are migrated; `None` when the app is served as it is,
    /// or has no good version to keep.
    pub serving: Option<String>,
    /// The collections whose records are being taken up to the version the
    /// app now declares.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub migrating: Vec<String>,
}

#[derive(Default)]
struct State {
    served: BTreeMap<String, Served>,
    checked: BTreeMap<String, Checked>,
    /// The manifests that passed and wait for their records' migration.
    pending: BTreeMap<String, Arc<Manifest>>,
}

#[derive(Clone)]
pub struct Apps {
    /// The live checkout.
    root: PathBuf,
    /// Its `apps/`.
    dir: PathBuf,
    /// Where the apps' records are, to see whether any is behind.
    store: Arc<Store>,
    state: Arc<RwLock<State>>,
    /// Woken when an app waits for its records' migration.
    migrations: Arc<Notify>,
}

/// The commit the checkout at `root` holds, when it is one.
fn head(root: &Path) -> Option<String> {
    git(root, &["rev-parse", "HEAD"]).ok()
}

impl Apps {
    /// The apps of the live checkout at `root`, under its `apps/`, with
    /// their records in `store`, read now.
    pub fn open(root: impl Into<PathBuf>, store: Arc<Store>) -> Apps {
        let root = root.into();
        let apps = Apps {
            dir: root.join("apps"),
            root,
            store,
            state: Arc::default(),
            migrations: Arc::default(),
        };
        apps.reload();
        apps
    }

    /// The live checkout the apps are part of.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The app folders under `apps/` now, archived or not.
    pub fn present(&self) -> Vec<String> {
        folders(&self.dir)
    }

    /// The commit the live checkout holds.
    pub fn head(&self) -> Option<String> {
        head(&self.root)
    }

    /// What wakes the migrations' worker.
    pub fn migrations(&self) -> Arc<Notify> {
        self.migrations.clone()
    }

    /// The manifests that passed and wait for their records' migration.
    pub fn pending(&self) -> Vec<Arc<Manifest>> {
        let state = self.state.read().unwrap_or_else(|e| e.into_inner());
        state.pending.values().cloned().collect()
    }

    /// Read and check every app again: one that passes is served as it now
    /// is; one refused keeps the version it had, if it had one.
    pub fn reload(&self) {
        let root = self.root().to_path_buf();
        let commit = head(&root);
        let archived = ids(&self.store.conn()).unwrap_or_else(|e| {
            tracing::error!(target: "yenbot::apps", "could not read the archived apps: {e:#}");
            BTreeSet::new()
        });
        // An archived app is not served, whatever its folder holds (§20.1).
        let ids: Vec<String> = folders(&self.dir)
            .into_iter()
            .filter(|id| !archived.contains(id))
            .collect();
        let mut state = self.state.write().unwrap_or_else(|e| e.into_inner());
        let mut served = BTreeMap::new();
        let mut checked = BTreeMap::new();
        let mut pending = BTreeMap::new();
        for id in ids {
            let was = state.served.get(&id);
            let refusals = match from_disk(&root, &id) {
                // A folder with nothing in it is no app.
                Ok(files) if files.names_under(&format!("apps/{id}/")).next().is_none() => continue,
                Ok(files) => check_app(
                    &files,
                    &id,
                    &Before {
                        manifest: was.map(|s| s.manifest.as_ref()),
                    },
                ),
                Err(e) => vec![Refusal::new(
                    "manifest",
                    format!("apps/{id}/"),
                    None,
                    format!("could not be read: {e:#}"),
                )],
            };
            if refusals.is_empty() {
                // The check read and took the manifest, so it parses.
                let text = std::fs::read_to_string(self.dir.join(&id).join("app.yaml"))
                    .unwrap_or_default();
                let Ok(m) = Manifest::parse(&text, &id) else {
                    continue;
                };
                let migrating = behind(&self.store.conn(), &m).unwrap_or_else(|e| {
                    tracing::error!(target: "yenbot::apps", app = %id, "could not see whether its records are behind: {e:#}");
                    Vec::new()
                });
                let mut serving = None;
                if migrating.is_empty() {
                    let now = Served {
                        manifest: Arc::new(m),
                        good: commit.clone(),
                        kept: false,
                    };
                    served.insert(id.clone(), now);
                } else {
                    // The version it had is served, from the commit it had it
                    // at, until its records are taken up to this one.
                    if let Some(was) = was {
                        serving = was.good.clone();
                        let kept = Served {
                            kept: true,
                            ..was.clone()
                        };
                        served.insert(id.clone(), kept);
                    }
                    tracing::info!(
                        target: "yenbot::apps",
                        app = %id,
                        collections = ?migrating,
                        "its records are being migrated; {}",
                        if serving.is_some() { "its last good version is served meanwhile" } else { "it opens once they are" }
                    );
                    pending.insert(id.clone(), Arc::new(m));
                }
                checked.insert(
                    id,
                    Checked {
                        commit: commit.clone(),
                        serving,
                        migrating,
                        ..Checked::default()
                    },
                );
                continue;
            }
            let kept = was.map(|s| Served {
                kept: true,
                ..s.clone()
            });
            let serving = kept.as_ref().and_then(|s| s.good.clone());
            tracing::warn!(
                target: "yenbot::apps",
                app = %id,
                serving = serving.as_deref().unwrap_or("nothing"),
                "check refused the app as it now is; {}:\n{}",
                if kept.is_some() { "its last good version is served" } else { "it is not served" },
                told(&refusals)
            );
            if let Some(kept) = kept {
                served.insert(id.clone(), kept);
            }
            checked.insert(
                id,
                Checked {
                    commit: commit.clone(),
                    refusals,
                    serving,
                    migrating: Vec::new(),
                },
            );
        }
        let wanted = !pending.is_empty();
        *state = State {
            served,
            checked,
            pending,
        };
        drop(state);
        if wanted {
            self.migrations.notify_one();
        }
    }

    pub fn get(&self, id: &str) -> Option<Arc<Manifest>> {
        self.state
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .served
            .get(id)
            .map(|s| s.manifest.clone())
    }

    pub fn all(&self) -> Vec<Arc<Manifest>> {
        self.state
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .served
            .values()
            .map(|s| s.manifest.clone())
            .collect()
    }

    /// Where app `id`'s files are served from, when it is served.
    pub fn files_at(&self, id: &str) -> Option<FilesAt> {
        let state = self.state.read().unwrap_or_else(|e| e.into_inner());
        let s = state.served.get(id)?;
        Some(match (&s.good, s.kept) {
            (Some(good), true) => FilesAt::Commit(good.clone()),
            _ => FilesAt::Tree,
        })
    }

    /// App `id`'s last check, for every folder under `apps/` — one refused
    /// and not served included.
    pub fn checked(&self, id: &str) -> Option<Checked> {
        self.state
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .checked
            .get(id)
            .cloned()
    }

    /// Apps held in memory, for tests.
    #[cfg(test)]
    pub fn of(manifests: Vec<Manifest>) -> Apps {
        let apps = Apps {
            root: PathBuf::new(),
            dir: PathBuf::new(),
            store: Arc::new(Store::in_memory().expect("a store in memory opens")),
            state: Arc::default(),
            migrations: Arc::default(),
        };
        let mut state = apps.state.write().unwrap();
        for m in manifests {
            state.checked.insert(m.id.clone(), Checked::default());
            state.served.insert(
                m.id.clone(),
                Served {
                    manifest: Arc::new(m),
                    good: None,
                    kept: false,
                },
            );
        }
        drop(state);
        apps
    }
}

/// The app folders under `dir`, by name.
fn folders(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<String> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().to_str().map(str::to_string))
        .filter(|name| !name.starts_with('.'))
        .collect();
    out.sort();
    out
}

#[cfg(test)]
mod tests;
