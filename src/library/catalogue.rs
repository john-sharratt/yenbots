//! `library/catalogue.yaml` (§15.2): one entry per picture, keyed by its kind
//! and id, with its tags and the prompt and seed that drew it, so any picture
//! can be drawn again in kind.
//!
//! ```yaml
//! tiles/bell-coral:
//!   tags: [bell, reminders]
//!   seed: 48151623
//!   prompt: >-
//!     A flat vector illustration of …
//! backgrounds/paper-warm:
//!   tags: [paper]
//!   tiling: true
//!   seed: 4242
//!   prompt: …
//! ```

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::kind::Kind;

/// The catalogue's path, from the repository's root.
pub const PATH: &str = "library/catalogue.yaml";

/// One picture's entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Entry {
    pub kind: Kind,
    pub id: String,
    pub tags: Vec<String>,
    pub prompt: String,
    pub seed: u64,
    /// A background that repeats edge to edge.
    pub tiling: bool,
    /// Its line in the catalogue, from 1.
    #[serde(skip)]
    pub line: usize,
}

impl Entry {
    /// `<kind>/<id>`: how an app and a URL name it.
    pub fn key(&self) -> String {
        format!("{}/{}", self.kind.name(), self.id)
    }

    /// The committed picture, from the repository's root.
    pub fn path(&self) -> String {
        format!("library/{}.png", self.key())
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Line {
    tags: Vec<String>,
    prompt: String,
    seed: u64,
    #[serde(default)]
    tiling: bool,
}

/// What is wrong with a catalogue: the line, when it is one line, and what.
pub type Fault = (Option<usize>, String);

/// Whether `word` is lowercase letters and digits joined by single `-`.
pub fn is_word(word: &str) -> bool {
    !word.is_empty()
        && !word.starts_with('-')
        && !word.ends_with('-')
        && !word.contains("--")
        && word
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// The line of `text` whose key is `key`, from 1.
fn line_of_key(text: &str, key: &str) -> usize {
    let want = format!("{key}:");
    text.lines()
        .position(|l| l.starts_with(&want))
        .map_or(1, |i| i + 1)
}

/// Every entry of the catalogue `text`, in key order — or everything wrong
/// with it.
pub fn parse(text: &str) -> Result<Vec<Entry>, Vec<Fault>> {
    let lines: BTreeMap<String, Line> = if text.trim().is_empty() {
        BTreeMap::new()
    } else {
        serde_yaml::from_str(text).map_err(|e| {
            vec![(
                e.location().map(|l| l.line()),
                format!("does not parse: {e}"),
            )]
        })?
    };
    let mut entries = Vec::new();
    let mut faults = Vec::new();
    for (key, l) in lines {
        let line = line_of_key(text, &key);
        let mut fault = |what: String| faults.push((Some(line), what));
        let Some((kind, id)) = key.split_once('/') else {
            fault(format!("`{key}` is not `<kind>/<id>`"));
            continue;
        };
        let Some(kind) = Kind::parse(kind) else {
            fault(format!(
                "`{key}`: `{kind}` is not a kind; the kinds are tiles, illustrations, empty, backgrounds, stickers"
            ));
            continue;
        };
        if !is_word(id) {
            fault(format!(
                "`{key}`: an id is lowercase letters and digits joined by `-`"
            ));
            continue;
        }
        if l.tags.is_empty() {
            fault(format!("`{key}` has no tags"));
        }
        for t in &l.tags {
            if !is_word(t) {
                fault(format!(
                    "`{key}`: the tag `{t}` is not lowercase letters and digits joined by `-`"
                ));
            }
        }
        if l.prompt.trim().is_empty() {
            fault(format!("`{key}` has no prompt"));
        }
        if l.tiling && kind != Kind::Backgrounds {
            fault(format!("`{key}`: only a background tiles"));
        }
        entries.push(Entry {
            kind,
            id: id.to_string(),
            tags: l.tags,
            prompt: l.prompt.trim().to_string(),
            seed: l.seed,
            tiling: l.tiling,
            line,
        });
    }
    if faults.is_empty() {
        Ok(entries)
    } else {
        Err(faults)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TWO: &str = "tiles/bell-coral:\n  tags: [bell, reminders]\n  seed: 48151623\n  prompt: >-\n    A bell\n    on coral.\nbackgrounds/paper-warm:\n  tags: [paper]\n  tiling: true\n  seed: 4242\n  prompt: Warm paper.\n";

    #[test]
    fn a_catalogue_reads_each_entry_with_its_line() {
        let got = parse(TWO).unwrap();
        assert_eq!(
            got,
            [
                Entry {
                    kind: Kind::Backgrounds,
                    id: "paper-warm".into(),
                    tags: vec!["paper".into()],
                    prompt: "Warm paper.".into(),
                    seed: 4242,
                    tiling: true,
                    line: 7,
                },
                Entry {
                    kind: Kind::Tiles,
                    id: "bell-coral".into(),
                    tags: vec!["bell".into(), "reminders".into()],
                    prompt: "A bell on coral.".into(),
                    seed: 48151623,
                    tiling: false,
                    line: 1,
                },
            ]
        );
        assert_eq!(got[1].key(), "tiles/bell-coral");
        assert_eq!(got[1].path(), "library/tiles/bell-coral.png");
        assert_eq!(parse("").unwrap(), Vec::new());
        assert_eq!(parse("# nothing drawn yet\n").unwrap(), Vec::new());
    }

    #[test]
    fn a_bad_entry_is_refused_on_its_line() {
        let bad = "tiles/Bell:\n  tags: [bell]\n  seed: 1\n  prompt: p\nhats/top:\n  tags: [hat]\n  seed: 1\n  prompt: p\nstickers/star:\n  tags: [Gold Star]\n  seed: 1\n  prompt: \" \"\n  tiling: true\nnokind:\n  tags: [x]\n  seed: 1\n  prompt: p\n";
        assert_eq!(
            parse(bad).unwrap_err(),
            [
                (Some(5), "`hats/top`: `hats` is not a kind; the kinds are tiles, illustrations, empty, backgrounds, stickers".to_string()),
                (Some(14), "`nokind` is not `<kind>/<id>`".to_string()),
                (Some(9), "`stickers/star`: the tag `Gold Star` is not lowercase letters and digits joined by `-`".to_string()),
                (Some(9), "`stickers/star` has no prompt".to_string()),
                (Some(9), "`stickers/star`: only a background tiles".to_string()),
                (Some(1), "`tiles/Bell`: an id is lowercase letters and digits joined by `-`".to_string()),
            ]
        );
    }

    #[test]
    fn a_catalogue_that_does_not_parse_says_where() {
        let faults = parse("tiles/bell:\n  tags: [bell\n  seed: 1\n").unwrap_err();
        assert_eq!(faults.len(), 1);
        // serde_yaml finds the unclosed `[` where the next key begins.
        assert_eq!(faults[0].0, Some(3));
        assert!(
            faults[0].1.starts_with("does not parse: "),
            "{}",
            faults[0].1
        );
        let unknown = parse("tiles/bell:\n  tags: [bell]\n  seed: 1\n  prompt: p\n  colour: red\n")
            .unwrap_err();
        assert!(
            unknown[0].1.contains("unknown field `colour`"),
            "{unknown:?}"
        );
    }

    #[test]
    fn a_word_is_lowercase_joined_by_dashes() {
        for good in ["bell", "bell-coral", "no-recipes-2"] {
            assert!(is_word(good), "{good}");
        }
        for bad in [
            "",
            "-bell",
            "bell-",
            "bell--coral",
            "Bell",
            "bell coral",
            "bell_coral",
        ] {
            assert!(!is_word(bad), "{bad}");
        }
    }
}
