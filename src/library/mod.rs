//! The art library (`candle/docs/yenbot_app_host.md` §15): the pictures
//! every app draws from, committed under the live checkout's `library/`
//! with one line each in `library/catalogue.yaml`.
//!
//! On load, and each time the checkout moves, every picture is sized to its
//! kind's sizes ([`sizing`]) and its dominant colour read ([`dominant`]);
//! the results are cached under `<data_dir>/library/<digest>/`, keyed by the
//! picture's content, so a picture is sized once whatever moves around it.
//! A catalogue that does not parse leaves the library as it was, and a line
//! whose picture is missing or unreadable is left out; the gate refuses
//! both before they land ([`check`]).

pub mod catalogue;
pub mod check;
pub mod dominant;
pub mod kind;
pub mod sizing;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use anyhow::{Context, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::apps::theme::colour::Colour;
use catalogue::Entry;
use dominant::{dominant, nearest};
use kind::Kind;
use sizing::{decode, sized, webp};

/// One picture as it is served.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    pub entry: Entry,
    /// The first 16 hex digits of the committed file's SHA-256.
    pub digest: String,
    pub colour: Colour,
    /// The palette colour nearest [`Picture::colour`].
    pub palette: &'static str,
}

/// A size of a picture, as the API answers it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Size {
    pub width: u32,
    pub height: u32,
    pub url: String,
}

/// A picture as the API answers it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Shown {
    pub id: String,
    pub kind: Kind,
    pub tags: Vec<String>,
    pub colour: String,
    pub palette: &'static str,
    pub tiling: bool,
    pub sizes: Vec<Size>,
    pub prompt: String,
    pub seed: u64,
}

impl Picture {
    /// Where its size `width` is served; `?v=` changes when it is redrawn.
    pub fn url(&self, width: u32) -> String {
        format!(
            "/_library/{}-{width}.webp?v={}",
            self.entry.key(),
            self.digest
        )
    }

    pub fn shown(&self) -> Shown {
        let e = &self.entry;
        Shown {
            id: e.key(),
            kind: e.kind,
            tags: e.tags.clone(),
            colour: self.colour.to_string(),
            palette: self.palette,
            tiling: e.tiling,
            sizes: e
                .kind
                .sizes()
                .iter()
                .map(|&(width, height)| Size {
                    width,
                    height,
                    url: self.url(width),
                })
                .collect(),
            prompt: e.prompt.clone(),
            seed: e.seed,
        }
    }
}

/// What a search of the library asks for; an absent part matches anything.
#[derive(Debug, Clone, Default)]
pub struct Find {
    pub kind: Option<Kind>,
    pub tag: Option<String>,
    /// A palette colour by name.
    pub colour: Option<String>,
}

#[derive(Clone)]
pub struct Library {
    /// The live checkout.
    root: PathBuf,
    /// Where sized pictures are kept.
    cache: PathBuf,
    pictures: Arc<RwLock<Arc<BTreeMap<String, Picture>>>>,
}

/// The cached file of size `w`×`h`.
fn size_file(dir: &Path, w: u32, h: u32) -> PathBuf {
    dir.join(format!("{w}x{h}.webp"))
}

/// Write `bytes` to `path` whole or not at all: to a file beside it, then
/// renamed over it.
fn write_whole(path: &Path, bytes: &[u8]) -> Result<()> {
    let part = path.with_extension("part");
    std::fs::write(&part, bytes)?;
    std::fs::rename(&part, path)?;
    Ok(())
}

impl Library {
    /// The library of the checkout at `root`, cached under `cache`; read by
    /// [`Library::reload`].
    pub fn new(root: impl Into<PathBuf>, cache: impl Into<PathBuf>) -> Library {
        Library {
            root: root.into(),
            cache: cache.into(),
            pictures: Arc::default(),
        }
    }

    /// Read the catalogue and size every picture not yet sized.
    pub fn reload(&self) {
        let path = self.root.join(catalogue::PATH);
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => {
                tracing::warn!(target: "yenbot::library", "{}: {e}; the library is kept as it was", catalogue::PATH);
                return;
            }
        };
        let entries = match catalogue::parse(&text) {
            Ok(e) => e,
            Err(faults) => {
                let said: Vec<String> = faults
                    .iter()
                    .map(|(line, what)| match line {
                        Some(l) => format!("{}:{l} {what}", catalogue::PATH),
                        None => format!("{} {what}", catalogue::PATH),
                    })
                    .collect();
                tracing::warn!(target: "yenbot::library", "the catalogue is refused; the library is kept as it was:\n{}", said.join("\n"));
                return;
            }
        };
        let mut pictures = BTreeMap::new();
        for entry in entries {
            match self.prepare(&entry) {
                Ok(p) => {
                    pictures.insert(entry.key(), p);
                }
                Err(e) => {
                    tracing::warn!(target: "yenbot::library", picture = %entry.key(), "left out: {e:#}");
                }
            }
        }
        self.sweep(&pictures);
        tracing::info!(target: "yenbot::library", pictures = pictures.len(), "library read");
        *self.pictures.write().unwrap_or_else(|e| e.into_inner()) = Arc::new(pictures);
    }

    /// `entry`'s picture, sized and its colour read — from the cache when
    /// this content was sized before.
    fn prepare(&self, entry: &Entry) -> Result<Picture> {
        let file = self.root.join(entry.path());
        let png = std::fs::read(&file).with_context(|| format!("reading {}", entry.path()))?;
        let digest: String = Sha256::digest(&png)[..8]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let dir = self.cache.join(&digest);
        let colour_file = dir.join("colour");
        let sizes = entry.kind.sizes();
        let cached = std::fs::read_to_string(&colour_file)
            .ok()
            .and_then(|c| Colour::parse(&c))
            .filter(|_| sizes.iter().all(|&(w, h)| size_file(&dir, w, h).is_file()));
        let colour = match cached {
            Some(c) => c,
            None => {
                let img = decode(&png).with_context(|| entry.path())?;
                std::fs::create_dir_all(&dir)?;
                for &(w, h) in sizes {
                    write_whole(&size_file(&dir, w, h), &webp(&sized(&img, w, h))?)?;
                }
                let c = dominant(&img).context("the picture is wholly transparent")?;
                write_whole(&colour_file, c.to_string().as_bytes())?;
                c
            }
        };
        Ok(Picture {
            entry: entry.clone(),
            digest,
            colour,
            palette: nearest(colour),
        })
    }

    /// Remove the cache of every picture no longer in the library.
    fn sweep(&self, pictures: &BTreeMap<String, Picture>) {
        let kept: BTreeSet<&str> = pictures.values().map(|p| p.digest.as_str()).collect();
        let Ok(dirs) = std::fs::read_dir(&self.cache) else {
            return;
        };
        for d in dirs.flatten() {
            let name = d.file_name().to_string_lossy().to_string();
            if d.path().is_dir() && !kept.contains(name.as_str()) {
                let _ = std::fs::remove_dir_all(d.path());
            }
        }
    }

    /// The picture `key` (`tiles/bell-coral`).
    pub fn get(&self, key: &str) -> Option<Picture> {
        self.pictures
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(key)
            .cloned()
    }

    /// The pictures matching `find`, in key order.
    pub fn find(&self, find: &Find) -> Vec<Picture> {
        let all = self
            .pictures
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        all.values()
            .filter(|p| find.kind.is_none_or(|k| p.entry.kind == k))
            .filter(|p| {
                find.tag
                    .as_ref()
                    .is_none_or(|t| p.entry.tags.iter().any(|x| x == t))
            })
            .filter(|p| find.colour.as_deref().is_none_or(|c| p.palette == c))
            .cloned()
            .collect()
    }

    /// The sized file a request names — `<kind>/<id>-<width>.webp` — when
    /// the picture and the size are there.
    pub fn file(&self, kind: &str, file: &str) -> Option<PathBuf> {
        let (id, width) = file.strip_suffix(".webp")?.rsplit_once('-')?;
        let width: u32 = width.parse().ok()?;
        let p = self.get(&format!("{kind}/{id}"))?;
        let &(w, h) = p.entry.kind.sizes().iter().find(|s| s.0 == width)?;
        Some(size_file(&self.cache.join(&p.digest), w, h))
    }
}

#[cfg(test)]
mod tests;
