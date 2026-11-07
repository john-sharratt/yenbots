//! An app's files (`candle/docs/yenbot_app_host.md` §6.1, §7.3, §12.6): a
//! picture or a PDF uploaded by a page or drawn by the image engine, kept
//! under `<data_dir>/files/<app>/<id>` with its row in `files`, and — for a
//! picture — a thumbnail, re-encoded, which drops anything hidden in the
//! upload. A record names a file by its id, in a `file` field.
//!
//! A file is never changed once kept, so it is served with a long cache and
//! copied once into the backups' own folder (§7.4).

pub mod kind;
pub mod mirror;

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use image::imageops::FilterType;
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::Serialize;
use serde_json::{json, Value};

use crate::data::docs::new_id;
use crate::library::sizing::webp;
use kind::FileKind;

/// The most a file may hold.
pub const MAX_BYTES: usize = 20 << 20;

/// The longest side of a thumbnail.
pub const THUMB: u32 = 480;

/// A file kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Kept {
    pub id: String,
    pub app: String,
    pub owner: String,
    pub name: String,
    pub kind: FileKind,
    pub bytes: u64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub created_ms: i64,
}

impl Kept {
    /// Where it is served.
    pub fn url(&self) -> String {
        format!("/api/v1/files/{}/{}", self.app, self.id)
    }

    /// Where its thumbnail is served, for a picture.
    pub fn thumb_url(&self) -> Option<String> {
        self.kind
            .is_picture()
            .then(|| format!("{}/thumb", self.url()))
    }

    /// As the API answers it.
    pub fn shown(&self) -> Value {
        json!({
            "id": self.id, "name": self.name, "type": self.kind.content_type(),
            "bytes": self.bytes, "width": self.width, "height": self.height,
            "url": self.url(), "thumb": self.thumb_url(), "created_ms": self.created_ms,
        })
    }
}

/// Where an app's files are, and the folder for each.
#[derive(Clone)]
pub struct Files {
    dir: PathBuf,
}

/// Write `bytes` to `path` whole or not at all.
fn write_whole(path: &Path, bytes: &[u8]) -> Result<()> {
    let part = path.with_extension("part");
    std::fs::write(&part, bytes).with_context(|| format!("writing {}", part.display()))?;
    std::fs::rename(&part, path).with_context(|| format!("naming {}", path.display()))?;
    Ok(())
}

/// The thumbnail of picture `bytes`, and its full size.
fn thumbnail(bytes: &[u8]) -> Result<(Vec<u8>, u32, u32)> {
    let img = image::load_from_memory(bytes).context("the picture cannot be read")?;
    let (w, h) = (img.width(), img.height());
    let small = if w.max(h) > THUMB {
        img.resize(THUMB, THUMB, FilterType::Lanczos3)
    } else {
        img
    };
    Ok((webp(&small.to_rgba8())?, w, h))
}

fn kept(r: &Row<'_>) -> rusqlite::Result<Kept> {
    let kind: String = r.get(4)?;
    Ok(Kept {
        id: r.get(0)?,
        app: r.get(1)?,
        owner: r.get(2)?,
        name: r.get(3)?,
        kind: FileKind::parse(&kind).unwrap_or(FileKind::Pdf),
        bytes: r.get(5)?,
        width: r.get(6)?,
        height: r.get(7)?,
        created_ms: r.get(8)?,
    })
}

impl Files {
    /// The files kept under `dir` (`<data_dir>/files`).
    pub fn new(dir: impl Into<PathBuf>) -> Files {
        Files { dir: dir.into() }
    }

    /// File `id` of `app`'s bytes on disk; `thumb` for its thumbnail.
    pub fn path(&self, app: &str, id: &str, thumb: bool) -> PathBuf {
        let name = if thumb {
            format!("{id}.thumb.webp")
        } else {
            id.to_string()
        };
        self.dir.join(app).join(name)
    }

    /// Keep `bytes` as a file of `app`, owned by `owner`, named `name`.
    pub fn keep(
        &self,
        conn: &Connection,
        app: &str,
        owner: &str,
        name: &str,
        bytes: &[u8],
        now: i64,
    ) -> Result<Kept> {
        if bytes.len() > MAX_BYTES {
            bail!("a file holds at most {} MB", MAX_BYTES >> 20);
        }
        let Some(kind) = FileKind::of(bytes) else {
            bail!("a file is a picture (PNG, JPEG, WebP, GIF) or a PDF");
        };
        let id = new_id(now);
        std::fs::create_dir_all(self.dir.join(app))?;
        let (width, height) = if kind.is_picture() {
            let (thumb, w, h) = thumbnail(bytes)?;
            write_whole(&self.path(app, &id, true), &thumb)?;
            (Some(w), Some(h))
        } else {
            (None, None)
        };
        write_whole(&self.path(app, &id, false), bytes)?;
        let name: String = name.chars().filter(|c| !c.is_control()).take(200).collect();
        let k = Kept {
            id,
            app: app.to_string(),
            owner: owner.to_string(),
            name,
            kind,
            bytes: bytes.len() as u64,
            width,
            height,
            created_ms: now,
        };
        conn.execute(
            "INSERT INTO files (app, id, owner, name, kind, bytes, width, height, created_ms) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                k.app,
                k.id,
                k.owner,
                k.name,
                k.kind.name(),
                k.bytes,
                k.width,
                k.height,
                now
            ],
        )?;
        Ok(k)
    }

    /// File `id` of `app`, when it is there.
    pub fn get(&self, conn: &Connection, app: &str, id: &str) -> Result<Option<Kept>> {
        Ok(conn
            .query_row(
                "SELECT id, app, owner, name, kind, bytes, width, height, created_ms FROM files \
                 WHERE app = ?1 AND id = ?2",
                params![app, id],
                kept,
            )
            .optional()?)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use image::{ImageFormat, Rgba, RgbaImage};

    use super::*;
    use crate::store::Store;

    fn png(w: u32, h: u32) -> Vec<u8> {
        let mut out = Cursor::new(Vec::new());
        RgbaImage::from_pixel(w, h, Rgba([212, 87, 61, 255]))
            .write_to(&mut out, ImageFormat::Png)
            .unwrap();
        out.into_inner()
    }

    /// **A picture is kept with a thumbnail and its size; a PDF without; a
    /// file of another kind, or too large, is refused.**
    #[test]
    fn a_file_is_kept_by_its_kind() {
        let dir = tempfile::tempdir().unwrap();
        let files = Files::new(dir.path());
        let store = Store::in_memory().unwrap();
        let conn = store.conn();
        let k = files
            .keep(&conn, "recipes", "1", "phở.png", &png(1200, 600), 5)
            .unwrap();
        assert_eq!(
            (k.width, k.height, k.kind),
            (Some(1200), Some(600), FileKind::Png)
        );
        let thumb = image::open(files.path("recipes", &k.id, true)).unwrap();
        assert_eq!((thumb.width(), thumb.height()), (480, 240));
        assert_eq!(
            std::fs::read(files.path("recipes", &k.id, false)).unwrap(),
            png(1200, 600)
        );
        assert_eq!(files.get(&conn, "recipes", &k.id).unwrap(), Some(k.clone()));
        assert_eq!(
            k.thumb_url(),
            Some(format!("/api/v1/files/recipes/{}/thumb", k.id))
        );
        assert_eq!(
            files.get(&conn, "other", &k.id).unwrap(),
            None,
            "an app's own"
        );

        let pdf = files
            .keep(&conn, "recipes", "1", "menu.pdf", b"%PDF-1.7 ...", 6)
            .unwrap();
        assert_eq!((pdf.width, pdf.thumb_url()), (None, None));
        let refused = files
            .keep(&conn, "recipes", "1", "x.svg", b"<svg/>", 7)
            .unwrap_err();
        assert_eq!(
            refused.to_string(),
            "a file is a picture (PNG, JPEG, WebP, GIF) or a PDF"
        );
        let big = vec![b'%'; MAX_BYTES + 1];
        assert!(files.keep(&conn, "recipes", "1", "big", &big, 8).is_err());
    }
}
