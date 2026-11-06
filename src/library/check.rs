//! `library`: the art library's rule (§15.2), run by the gate on a change to
//! `library/`. The catalogue parses, every line's picture is there, every
//! picture has its line, and the library holds nothing else.

use std::collections::BTreeSet;

use crate::check::files::Files;
use crate::check::Refusal;

use super::catalogue::{self, is_word, PATH};
use super::kind::Kind;

pub const RULE: &str = "library";

/// The keys (`tiles/bell-coral`) of the catalogue `files` holds; empty when
/// it holds none, or one that does not parse — which [`check`] refuses.
pub fn keys(files: &Files) -> BTreeSet<String> {
    files
        .text
        .get(PATH)
        .and_then(|t| catalogue::parse(t).ok())
        .map(|entries| entries.iter().map(|e| e.key()).collect())
        .unwrap_or_default()
}

/// Every refusal of the library `files` holds.
pub fn check(files: &Files) -> Vec<Refusal> {
    let mut out = Vec::new();
    let text = files.text.get(PATH).map(String::as_str).unwrap_or("");
    let entries = match catalogue::parse(text) {
        Ok(e) => e,
        Err(faults) => {
            for (line, what) in faults {
                out.push(Refusal::new(RULE, PATH, line, what));
            }
            return out;
        }
    };
    let mut named = BTreeSet::new();
    for e in &entries {
        named.insert(e.path());
        if !files.has(&e.path()) {
            out.push(Refusal::new(
                RULE,
                PATH,
                Some(e.line),
                format!(
                    "names `{}`, but there is no {}; commit the picture, or take the line out",
                    e.key(),
                    e.path()
                ),
            ));
        }
    }
    for path in files.names_under("library/") {
        if path == PATH || named.contains(path) {
            continue;
        }
        let picture = path
            .strip_prefix("library/")
            .and_then(|p| p.strip_suffix(".png"))
            .and_then(|p| p.split_once('/'))
            .filter(|(kind, id)| Kind::parse(kind).is_some() && is_word(id));
        let message = match picture {
            Some((kind, id)) => format!(
                "has no line in {PATH}; add `{kind}/{id}:` with its tags, prompt and seed"
            ),
            None => "is not a picture of the library: it holds `<kind>/<id>.png` and catalogue.yaml alone".to_string(),
        };
        out.push(Refusal::new(RULE, path, None, message));
    }
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::check::files::from_disk;

    const ONE: &str = "tiles/bell-coral:\n  tags: [bell]\n  seed: 1\n  prompt: A bell.\nempty/no-recipes:\n  tags: [cooking]\n  seed: 2\n  prompt: An empty pot.\n";

    fn files(names: &[&str]) -> Files {
        let mut f = Files::default();
        f.add(PATH, ONE.as_bytes().to_vec());
        for n in names {
            f.name(n);
        }
        f
    }

    #[test]
    fn a_library_whose_lines_and_pictures_agree_passes() {
        let f = files(&[
            "library/tiles/bell-coral.png",
            "library/empty/no-recipes.png",
        ]);
        assert_eq!(check(&f), Vec::new());
        assert_eq!(
            keys(&f).into_iter().collect::<Vec<_>>(),
            ["empty/no-recipes", "tiles/bell-coral"]
        );
        assert_eq!(check(&Files::default()), Vec::new(), "no library yet");
    }

    #[test]
    fn a_line_without_its_picture_and_a_picture_without_its_line_are_refused() {
        let f = files(&[
            "library/tiles/bell-coral.png",
            "library/tiles/cup-teal.png",
            "library/tiles/notes.txt",
            "library/hats/top.png",
        ]);
        let told: Vec<String> = check(&f).iter().map(Refusal::to_string).collect();
        assert_eq!(
            told,
            [
                "library/catalogue.yaml:5 names `empty/no-recipes`, but there is no library/empty/no-recipes.png; commit the picture, or take the line out",
                "library/hats/top.png is not a picture of the library: it holds `<kind>/<id>.png` and catalogue.yaml alone",
                "library/tiles/cup-teal.png has no line in library/catalogue.yaml; add `tiles/cup-teal:` with its tags, prompt and seed",
                "library/tiles/notes.txt is not a picture of the library: it holds `<kind>/<id>.png` and catalogue.yaml alone",
            ]
        );
    }

    /// **The repository's own library passes**: every picture committed has
    /// its line, and every line its picture.
    #[test]
    fn the_repositorys_library_passes() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let files = from_disk(root, "no-app").unwrap();
        assert_eq!(check(&files), Vec::new());
        assert!(keys(&files).contains("tiles/bell-coral"));
    }

    #[test]
    fn a_catalogue_that_does_not_parse_is_refused_and_names_nothing() {
        let mut f = Files::default();
        f.add(
            PATH,
            b"tiles/bell:\n  tags: [bell]\n  seed: x\n  prompt: p\n".to_vec(),
        );
        let r = check(&f);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].path, PATH);
        assert!(
            r[0].message.starts_with("does not parse: "),
            "{}",
            r[0].message
        );
        assert!(keys(&f).is_empty());
    }
}
