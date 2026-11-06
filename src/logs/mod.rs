//! What happens on the site, written down where the connector reads it
//! (`candle/docs/yenbot_app_host.md` §18, `candle/docs/mcp_file_tools.md` §6):
//! the platform's own lines as source `yenbot`, and each app's — browser
//! errors, workflow runs, check refusals — as `yenbot/<app>`, kept in the
//! store for [`KEEP_DAYS`] and served as a log source ([`api`]).
//!
//! The platform's lines come from `tracing` ([`layer`]): every event at INFO
//! or above is a line, written off the request path by one writer thread.

pub mod api;
pub mod layer;

use std::sync::Arc;

use anyhow::Result;
use regex::Regex;
use rusqlite::params;
use serde::Serialize;

use crate::store::Store;

/// The platform's own source.
pub const PLATFORM: &str = "yenbot";

/// How long a line is kept.
pub const KEEP_DAYS: i64 = 30;

/// One line, as it is stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub at_ms: i64,
    pub level: Level,
    pub source: String,
    pub target: String,
    pub message: String,
}

/// A line's level, least to most severe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Level {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl Level {
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Trace => "TRACE",
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }

    pub fn parse(s: &str) -> Option<Level> {
        Some(match s.trim().to_ascii_uppercase().as_str() {
            "TRACE" => Level::Trace,
            "DEBUG" => Level::Debug,
            "INFO" => Level::Info,
            "WARN" => Level::Warn,
            "ERROR" => Level::Error,
            _ => return None,
        })
    }
}

/// `at_ms` as RFC 3339 in UTC, to the millisecond: `2026-11-06T10:02:11.042Z`.
pub fn rfc3339(at_ms: i64) -> String {
    let secs = at_ms.div_euclid(1000);
    let ms = at_ms.rem_euclid(1000);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (h, m, s) = (rem / 3600, rem % 3600 / 60, rem % 60);
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(mo <= 2);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}.{ms:03}Z")
}

impl Line {
    /// The line as a reader sees it: `<time> <LEVEL> <source> <target>: <message>`.
    pub fn render(&self) -> String {
        format!(
            "{} {} {} {}: {}",
            rfc3339(self.at_ms),
            self.level.as_str(),
            self.source,
            self.target,
            self.message
        )
    }
}

/// Lines written to and read from the store.
#[derive(Clone)]
pub struct Logs {
    store: Arc<Store>,
}

/// What a search asks for.
#[derive(Debug, Clone, Default)]
pub struct Search {
    pub grep: Option<Regex>,
    pub level: Option<Level>,
    /// A source, or every source when `None`; `yenbot` names the platform's
    /// own lines alone.
    pub source: Option<String>,
    /// Only lines rendered at or after this RFC 3339 prefix.
    pub since: Option<String>,
    pub limit: usize,
}

/// What a search found: the newest `limit` lines matching, oldest first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Found {
    pub lines: Vec<String>,
    pub matched: usize,
    pub complete: bool,
}

/// One source and what it holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceRow {
    pub name: String,
    pub lines: u64,
    pub newest: Option<String>,
}

impl Logs {
    pub fn new(store: Arc<Store>) -> Logs {
        Logs { store }
    }

    /// Write `lines`, and drop what is older than [`KEEP_DAYS`].
    pub fn write(&self, lines: &[Line]) -> Result<()> {
        let mut conn = self.store.conn();
        let tx = conn.transaction()?;
        for l in lines {
            tx.execute(
                "INSERT INTO log_lines (at_ms, level, source, target, message) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![l.at_ms, l.level.as_str(), l.source, l.target, l.message],
            )?;
        }
        if let Some(newest) = lines.iter().map(|l| l.at_ms).max() {
            tx.execute(
                "DELETE FROM log_lines WHERE at_ms < ?1",
                params![newest - KEEP_DAYS * 86_400_000],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// The lines `search` asks for: read newest first and stopped once it
    /// has `limit`, so a search over a month of lines reads only as far back
    /// as its answer needs.
    pub fn search(&self, search: &Search) -> Result<Found> {
        let conn = self.store.conn();
        let mut stmt = conn.prepare(
            "SELECT at_ms, level, source, target, message FROM log_lines \
             WHERE (?1 IS NULL OR source = ?1) ORDER BY at_ms DESC, id DESC",
        )?;
        let rows = stmt.query_map(params![search.source], |r| {
            Ok(Line {
                at_ms: r.get(0)?,
                level: Level::parse(&r.get::<_, String>(1)?).unwrap_or(Level::Info),
                source: r.get(2)?,
                target: r.get(3)?,
                message: r.get(4)?,
            })
        })?;
        let mut kept = Vec::new();
        let mut complete = true;
        for row in rows {
            let line = row?;
            let text = line.render();
            if let Some(since) = &search.since {
                if text.as_str() < since.as_str() {
                    break;
                }
            }
            if search.level.is_some_and(|l| line.level < l) {
                continue;
            }
            if search.grep.as_ref().is_some_and(|g| !g.is_match(&text)) {
                continue;
            }
            if kept.len() == search.limit {
                complete = false;
                break;
            }
            kept.push(text);
        }
        kept.reverse();
        Ok(Found {
            matched: kept.len(),
            lines: kept,
            complete,
        })
    }

    /// Every source, with its count and newest line.
    pub fn sources(&self) -> Result<Vec<SourceRow>> {
        let conn = self.store.conn();
        let mut stmt = conn.prepare(
            "SELECT source, COUNT(*), MAX(at_ms) FROM log_lines GROUP BY source ORDER BY source",
        )?;
        let mut rows: Vec<SourceRow> = stmt
            .query_map([], |r| {
                Ok(SourceRow {
                    name: r.get(0)?,
                    lines: r.get(1)?,
                    newest: r.get::<_, Option<i64>>(2)?.map(rfc3339),
                })
            })?
            .collect::<Result<_, _>>()?;
        if !rows.iter().any(|r| r.name == PLATFORM) {
            rows.insert(
                0,
                SourceRow {
                    name: PLATFORM.into(),
                    lines: 0,
                    newest: None,
                },
            );
        }
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(at_ms: i64, level: Level, source: &str, message: &str) -> Line {
        Line {
            at_ms,
            level,
            source: source.into(),
            target: "yenbot::t".into(),
            message: message.into(),
        }
    }

    fn logs() -> Logs {
        Logs::new(Arc::new(Store::in_memory().unwrap()))
    }

    #[test]
    fn a_time_renders_as_rfc3339_utc() {
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(rfc3339(1_793_959_331_042), "2026-11-06T10:02:11.042Z");
        assert_eq!(rfc3339(951_782_400_000), "2000-02-29T00:00:00.000Z");
    }

    #[test]
    fn a_line_renders_with_its_time_level_and_source() {
        assert_eq!(
            line(
                0,
                Level::Warn,
                "yenbot/recipes",
                "TypeError: x is undefined"
            )
            .render(),
            "1970-01-01T00:00:00.000Z WARN yenbot/recipes yenbot::t: TypeError: x is undefined"
        );
    }

    /// **A search keeps the newest matches, oldest first**, and says when it
    /// stopped before reading everything.
    #[test]
    fn a_search_keeps_the_newest_matches_oldest_first() {
        let l = logs();
        l.write(&[
            line(1_000, Level::Info, "yenbot", "started"),
            line(2_000, Level::Error, "yenbot/recipes", "TypeError one"),
            line(3_000, Level::Warn, "yenbot/recipes", "slow"),
            line(4_000, Level::Error, "yenbot/recipes", "TypeError two"),
        ])
        .unwrap();
        let all = l
            .search(&Search {
                limit: 10,
                ..Search::default()
            })
            .unwrap();
        assert_eq!((all.lines.len(), all.complete), (4, true));
        assert!(all.lines[0].ends_with("started"));

        let errors = l
            .search(&Search {
                grep: Some(Regex::new("TypeError").unwrap()),
                source: Some("yenbot/recipes".into()),
                limit: 1,
                ..Search::default()
            })
            .unwrap();
        assert_eq!(errors.lines.len(), 1);
        assert!(errors.lines[0].ends_with("TypeError two"));
        assert!(!errors.complete, "an earlier match was left out");

        let warn = l
            .search(&Search {
                level: Some(Level::Warn),
                since: Some("1970-01-01T00:00:03".into()),
                limit: 10,
                ..Search::default()
            })
            .unwrap();
        assert_eq!(warn.lines.len(), 2);
        assert!(warn.lines[0].ends_with("slow"));
    }

    #[test]
    fn the_sources_name_the_platform_and_each_app() {
        let l = logs();
        assert_eq!(
            l.sources().unwrap(),
            vec![SourceRow {
                name: "yenbot".into(),
                lines: 0,
                newest: None
            }]
        );
        l.write(&[line(5_000, Level::Error, "yenbot/recipes", "x")])
            .unwrap();
        let rows = l.sources().unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1].name, "yenbot/recipes");
        assert_eq!(rows[1].newest.as_deref(), Some("1970-01-01T00:00:05.000Z"));
    }

    #[test]
    fn lines_older_than_a_month_are_dropped() {
        let l = logs();
        l.write(&[line(0, Level::Info, "yenbot", "old")]).unwrap();
        l.write(&[line(
            KEEP_DAYS * 86_400_000 + 1,
            Level::Info,
            "yenbot",
            "new",
        )])
        .unwrap();
        let found = l
            .search(&Search {
                limit: 10,
                ..Search::default()
            })
            .unwrap();
        assert_eq!(found.lines.len(), 1);
    }
}
