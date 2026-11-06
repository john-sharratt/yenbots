//! The files the check reads, from a folder on disk or from a commit: an
//! app's folder (`apps/<id>/`), with its text files read and every other file
//! known by name; the art library by name (`library/`) with its catalogue
//! read, for the pictures an app names; and the app-building guide
//! (`docs/building-an-app.md`), for its edition. Paths are from the
//! repository's root, in `/`.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{anyhow, bail, Context, Result};

use crate::git::git_bytes;
use crate::library::catalogue::PATH as CATALOGUE;

/// The guide every app is built to (§17.3).
pub const GUIDE: &str = "docs/building-an-app.md";

/// Whether `path` is a file the check reads the text of, besides an app's
/// own: the guide and the library's catalogue.
fn read_whole(path: &str) -> bool {
    path == GUIDE || path == CATALOGUE
}

/// The files whose text the check reads; any other is known by name.
const TEXT: &[&str] = &[
    "js", "mjs", "css", "html", "yaml", "yml", "json", "md", "txt",
];

fn is_text(path: &str) -> bool {
    path.rsplit_once('.')
        .is_some_and(|(_, ext)| TEXT.contains(&ext.to_ascii_lowercase().as_str()))
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Files {
    /// Every file, by path.
    pub names: BTreeSet<String>,
    /// The text files' contents, by path; one that is not UTF-8 is absent
    /// here and named in [`Files::unreadable`].
    pub text: BTreeMap<String, String>,
    /// Text files that are not UTF-8.
    pub unreadable: BTreeSet<String>,
}

impl Files {
    /// Add the file at `path` with `bytes`.
    pub fn add(&mut self, path: &str, bytes: Vec<u8>) {
        self.names.insert(path.to_string());
        if is_text(path) {
            match String::from_utf8(bytes) {
                Ok(s) => {
                    self.text.insert(path.to_string(), s);
                }
                Err(_) => {
                    self.unreadable.insert(path.to_string());
                }
            }
        }
    }

    /// Add a file known by name only.
    pub fn name(&mut self, path: &str) {
        self.names.insert(path.to_string());
    }

    /// The text files under `prefix`, in path order.
    pub fn texts_under<'a>(
        &'a self,
        prefix: &'a str,
    ) -> impl Iterator<Item = (&'a str, &'a str)> + 'a {
        self.text
            .range(prefix.to_string()..)
            .take_while(move |(p, _)| p.starts_with(prefix))
            .map(|(p, t)| (p.as_str(), t.as_str()))
    }

    /// Every file under `prefix`, in path order.
    pub fn names_under<'a>(&'a self, prefix: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        self.names
            .range(prefix.to_string()..)
            .take_while(move |p| p.starts_with(prefix))
            .map(String::as_str)
    }

    pub fn has(&self, path: &str) -> bool {
        self.names.contains(path)
    }

    /// The edition the guide is at: its line `Edition: <n>`; `None` while
    /// there is no guide.
    pub fn guide_edition(&self) -> Option<u32> {
        self.text.get(GUIDE)?.lines().find_map(|l| {
            l.trim()
                .strip_prefix("Edition:")
                .and_then(|n| n.trim().parse().ok())
        })
    }
}

/// App `id`'s files, the library's names and the guide, from the checkout
/// at `root`.
pub fn from_disk(root: &Path, id: &str) -> Result<Files> {
    let mut files = Files::default();
    walk(root, &root.join("apps").join(id), &mut files, true)?;
    walk(root, &root.join("library"), &mut files, false)?;
    for path in [GUIDE, CATALOGUE] {
        let file = root.join(path);
        if file.is_file() {
            files.add(path, std::fs::read(&file)?);
        }
    }
    Ok(files)
}

fn walk(root: &Path, dir: &Path, files: &mut Files, read: bool) -> Result<()> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(());
    };
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            walk(root, &path, files, read)?;
            continue;
        }
        let rel = path
            .strip_prefix(root)?
            .to_string_lossy()
            .replace('\\', "/");
        if read {
            files.add(&rel, std::fs::read(&path)?);
        } else {
            files.name(&rel);
        }
    }
    Ok(())
}

/// App `id`'s files, the library's names and the guide, as `commit` holds
/// them in the repository at `repo` — read from its objects, never from a
/// working tree.
pub fn from_commit(repo: &Path, commit: &str, id: &str) -> Result<Files> {
    read_commit(repo, commit, Some(&format!("apps/{id}/")))
}

/// The library's names and its catalogue, and the guide, as `commit` holds
/// them: what the library's own check reads.
pub fn library_from_commit(repo: &Path, commit: &str) -> Result<Files> {
    read_commit(repo, commit, None)
}

fn read_commit(repo: &Path, commit: &str, app: Option<&str>) -> Result<Files> {
    let mut args = vec!["ls-tree", "-r", "-z", commit, "--"];
    args.extend(app);
    args.extend(["library", GUIDE]);
    let listed = git_bytes(repo, &args)?;
    // Each entry: "<mode> <type> <object>\t<path>\0".
    let mut wanted: Vec<(String, String)> = Vec::new();
    let mut files = Files::default();
    for entry in listed.split(|b| *b == 0).filter(|e| !e.is_empty()) {
        let entry = String::from_utf8_lossy(entry);
        let (meta, path) = entry.split_once('\t').context("an ls-tree line")?;
        let mut parts = meta.split(' ');
        let (_mode, kind, object) = (parts.next(), parts.next(), parts.next());
        if kind != Some("blob") {
            continue;
        }
        if app.is_some_and(|a| path.starts_with(a)) || read_whole(path) {
            wanted.push((
                path.to_string(),
                object.context("an object id")?.to_string(),
            ));
        } else {
            files.name(path);
        }
    }
    for (path, bytes) in blobs(repo, &wanted)? {
        files.add(&path, bytes);
    }
    Ok(files)
}

/// The contents of each `(path, object)`, read in one `git cat-file --batch`.
fn blobs(repo: &Path, wanted: &[(String, String)]) -> Result<Vec<(String, Vec<u8>)>> {
    if wanted.is_empty() {
        return Ok(Vec::new());
    }
    let mut child = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["cat-file", "--batch"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut input = child.stdin.take().context("cat-file's input")?;
    let ids: String = wanted.iter().map(|(_, o)| format!("{o}\n")).collect();
    let writer = std::thread::spawn(move || input.write_all(ids.as_bytes()));
    let mut out = Vec::new();
    child
        .stdout
        .take()
        .context("cat-file's output")?
        .read_to_end(&mut out)?;
    writer
        .join()
        .map_err(|_| anyhow!("cat-file's writer panicked"))??;
    let status = child.wait()?;
    if !status.success() {
        bail!("git cat-file --batch failed: {status}");
    }
    // Each answer: "<object> blob <size>\n<bytes>\n".
    let mut at = 0;
    let mut read = Vec::with_capacity(wanted.len());
    for (path, _) in wanted {
        let end = out[at..]
            .iter()
            .position(|b| *b == b'\n')
            .context("a cat-file header")?
            + at;
        let header = String::from_utf8_lossy(&out[at..end]).to_string();
        let size: usize = header
            .rsplit(' ')
            .next()
            .and_then(|s| s.parse().ok())
            .with_context(|| format!("a cat-file header: {header}"))?;
        let start = end + 1;
        read.push((path.clone(), out[start..start + size].to_vec()));
        at = start + size + 1;
    }
    Ok(read)
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

    fn put(root: &Path, path: &str, bytes: &[u8]) {
        let p = root.join(path);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, bytes).unwrap();
    }

    fn tree(root: &Path) {
        put(root, "apps/notes/app.yaml", b"id: notes\nname: Notes\n");
        put(root, "apps/notes/pages/home.js", b"export {};\n");
        put(root, "apps/notes/style.css", &[0xff, 0xfe]);
        put(root, "apps/other/app.yaml", b"id: other\nname: Other\n");
        put(root, "library/tiles/bell.png", &[0x89, b'P']);
        put(root, CATALOGUE, b"# the pictures\n");
        put(root, GUIDE, b"# Building an app\n\nEdition: 3\n");
    }

    fn library() -> Files {
        let mut f = Files::default();
        f.name("library/tiles/bell.png");
        f.add(CATALOGUE, b"# the pictures\n".to_vec());
        f.add(GUIDE, b"# Building an app\n\nEdition: 3\n".to_vec());
        f
    }

    fn expected() -> Files {
        let mut f = library();
        f.add("apps/notes/app.yaml", b"id: notes\nname: Notes\n".to_vec());
        f.add("apps/notes/pages/home.js", b"export {};\n".to_vec());
        f.add("apps/notes/style.css", vec![0xff, 0xfe]);
        f
    }

    /// **An app's files read the same from disk and from a commit**: its text,
    /// the library by name, the guide; another app's files are not read.
    #[test]
    fn an_apps_files_read_the_same_from_disk_and_from_a_commit() {
        let dir = tempfile::tempdir().unwrap();
        tree(dir.path());
        let disk = from_disk(dir.path(), "notes").unwrap();
        assert_eq!(disk, expected());
        assert_eq!(
            disk.unreadable.iter().collect::<Vec<_>>(),
            ["apps/notes/style.css"]
        );
        assert_eq!(disk.guide_edition(), Some(3));

        run(dir.path(), &["init", "-q", "-b", "main"]);
        run(dir.path(), &["add", "."]);
        run(dir.path(), &["commit", "-q", "-m", "one"]);
        let head = run(dir.path(), &["rev-parse", "HEAD"]);
        // A change on disk after the commit is not what the commit holds.
        put(dir.path(), "apps/notes/pages/home.js", b"changed\n");
        assert_eq!(from_commit(dir.path(), &head, "notes").unwrap(), expected());
        assert_eq!(library_from_commit(dir.path(), &head).unwrap(), library());
        assert_eq!(
            from_commit(dir.path(), &head, "absent")
                .unwrap()
                .names_under("apps/")
                .count(),
            0
        );
    }

    #[test]
    fn a_tree_with_no_guide_has_no_edition() {
        let mut f = Files::default();
        assert_eq!(f.guide_edition(), None);
        f.add(GUIDE, b"# Building an app\n".to_vec());
        assert_eq!(f.guide_edition(), None);
    }
}
