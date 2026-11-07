//! Nightly backups (`candle/docs/yenbot_app_host.md` §7.4): the store written
//! whole with `VACUUM INTO` to `<data_dir>/backups/yenbot-YYYY-MM-DD.db`,
//! named for the day in Yen's time zone, one a day, the newest [`KEEP`]
//! kept. `VACUUM INTO` writes a consistent copy while the site keeps
//! serving. The platform's `backup` job runs it at 03:00
//! ([`crate::jobs::platform`]).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use jiff::tz::TimeZone;
use jiff::Timestamp;
use serde::Serialize;

use super::Store;

/// How many backups are kept.
pub const KEEP: usize = 14;

/// The backup file for the day `at_ms` falls on in the zone `tz`.
pub fn file_for(dir: &Path, at_ms: i64, tz: &TimeZone) -> PathBuf {
    let day = Timestamp::from_millisecond(at_ms)
        .map(|t| t.to_zoned(tz.clone()).date().to_string())
        .unwrap_or_else(|_| "0000-00-00".into());
    dir.join(format!("yenbot-{day}.db"))
}

/// Write today's backup into `dir` unless it exists, then drop all but the
/// newest [`KEEP`]. Answers the file written, if one was.
pub fn back_up(store: &Store, dir: &Path, at_ms: i64, tz: &TimeZone) -> Result<Option<PathBuf>> {
    std::fs::create_dir_all(dir).with_context(|| format!("making {}", dir.display()))?;
    let file = file_for(dir, at_ms, tz);
    let written = if file.exists() {
        None
    } else {
        // Written beside its name and renamed into it, so a backup cut short
        // is never taken for the day's: the next look writes it again.
        let part = file.with_extension("db.part");
        if part.exists() {
            std::fs::remove_file(&part).with_context(|| format!("removing {}", part.display()))?;
        }
        store
            .conn()
            .execute("VACUUM INTO ?1", [part.to_string_lossy().to_string()])
            .with_context(|| format!("writing {}", part.display()))?;
        std::fs::rename(&part, &file).with_context(|| format!("naming {}", file.display()))?;
        tracing::info!(file = %file.display(), "store: backed up");
        Some(file)
    };
    let mut held: Vec<PathBuf> = std::fs::read_dir(dir)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("yenbot-") && n.ends_with(".db"))
        })
        .collect();
    held.sort();
    let surplus = held.len().saturating_sub(KEEP);
    for old in held.into_iter().take(surplus) {
        std::fs::remove_file(&old).with_context(|| format!("removing {}", old.display()))?;
    }
    Ok(written)
}

/// A backup held: its day and its size.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Held {
    pub date: String,
    pub bytes: u64,
}

/// The backups in `dir`, oldest first.
pub fn list(dir: &Path) -> Result<Vec<Held>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut held = Vec::new();
    for entry in std::fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        let date = name
            .strip_prefix("yenbot-")
            .and_then(|n| n.strip_suffix(".db"));
        if let Some(date) = date {
            if is_date(date) {
                held.push(Held {
                    date: date.to_string(),
                    bytes: entry.metadata()?.len(),
                });
            }
        }
    }
    held.sort_by(|a, b| a.date.cmp(&b.date));
    Ok(held)
}

/// Whether `s` is a day as backups are named: `YYYY-MM-DD`.
pub fn is_date(s: &str) -> bool {
    let fits = |(i, c): (usize, char)| match i {
        4 | 7 => c == '-',
        _ => c.is_ascii_digit(),
    };
    s.len() == 10 && s.char_indices().all(fits)
}

/// The backup file of the day `date`, if there is one.
pub fn held_file(dir: &Path, date: &str) -> Option<PathBuf> {
    let file = dir.join(format!("yenbot-{date}.db"));
    (is_date(date) && file.is_file()).then_some(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: i64 = 86_400_000;

    fn back_up(store: &Store, dir: &Path, at_ms: i64) -> Result<Option<PathBuf>> {
        super::back_up(store, dir, at_ms, &TimeZone::UTC)
    }

    fn file_for(dir: &Path, at_ms: i64) -> PathBuf {
        super::file_for(dir, at_ms, &TimeZone::UTC)
    }

    /// **A backup is named for its day in Yen's zone**: 03:00 on the 7th in
    /// Sydney is still the 6th in Greenwich.
    #[test]
    fn a_backup_is_named_by_its_day() {
        assert_eq!(
            file_for(Path::new("b"), 1_793_959_331_042),
            Path::new("b").join("yenbot-2026-11-06.db")
        );
        let sydney = TimeZone::get("Australia/Sydney").unwrap();
        // 2026-11-06T16:00Z is 03:00 on the 7th in Sydney.
        let three = 1_793_980_800_000;
        assert_eq!(
            super::file_for(Path::new("b"), three, &sydney),
            Path::new("b").join("yenbot-2026-11-07.db")
        );
        assert_eq!(
            file_for(Path::new("b"), three),
            Path::new("b").join("yenbot-2026-11-06.db")
        );
    }

    /// **One backup a day, a whole copy of the store, the newest fourteen
    /// kept.**
    #[test]
    fn one_backup_a_day_and_the_newest_fourteen_kept() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("yenbot.db")).unwrap();
        store
            .conn()
            .execute(
                "INSERT INTO accounts (id, provider, email, name, status, created_ms, seen_ms) \
                 VALUES ('1', 'google', 'a@b.c', 'A', 'active', 0, 0)",
                [],
            )
            .unwrap();
        let backups = dir.path().join("backups");
        let first = back_up(&store, &backups, 0).unwrap().unwrap();
        assert_eq!(
            back_up(&store, &backups, 1_000).unwrap(),
            None,
            "once a day"
        );
        let copy = Store::open(&first).unwrap();
        let n: u32 = copy
            .conn()
            .query_row("SELECT COUNT(*) FROM accounts", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1, "the copy holds the store's rows");
        drop(copy);
        for day in 1..20 {
            back_up(&store, &backups, day * DAY).unwrap();
        }
        let held = std::fs::read_dir(&backups).unwrap().count();
        assert_eq!(held, KEEP);
        assert!(!file_for(&backups, 0).exists(), "the oldest went first");
        assert!(file_for(&backups, 19 * DAY).exists());
    }

    /// **The backups are listed by day with their sizes**, and a day is
    /// named exactly — never a path.
    #[test]
    fn the_backups_are_listed_by_day() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("yenbot-2026-11-06.db"), b"12345").unwrap();
        std::fs::write(dir.path().join("yenbot-2026-11-05.db"), b"123").unwrap();
        std::fs::write(dir.path().join("yenbot-2026-11-07.db.part"), b"1").unwrap();
        std::fs::write(dir.path().join("replaced-1.db"), b"1").unwrap();
        let held = |date: &str, bytes| Held {
            date: date.into(),
            bytes,
        };
        assert_eq!(
            list(dir.path()).unwrap(),
            [held("2026-11-05", 3), held("2026-11-06", 5)]
        );
        assert_eq!(list(&dir.path().join("none")).unwrap(), []);
        assert!(held_file(dir.path(), "2026-11-06").is_some());
        assert!(held_file(dir.path(), "2026-11-04").is_none());
        assert!(held_file(dir.path(), "../yenbot").is_none());
        assert!(!is_date("2026-1-061"));
    }

    /// **A backup cut short is written again**, and is never one of the
    /// fourteen.
    #[test]
    fn a_backup_cut_short_is_written_again() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("yenbot.db")).unwrap();
        let backups = dir.path().join("backups");
        std::fs::create_dir_all(&backups).unwrap();
        let part = file_for(&backups, 0).with_extension("db.part");
        std::fs::write(&part, b"cut short").unwrap();
        assert_eq!(
            back_up(&store, &backups, 0).unwrap(),
            Some(file_for(&backups, 0))
        );
        assert!(!part.exists());
        Store::open(&file_for(&backups, 0)).unwrap();
    }
}
