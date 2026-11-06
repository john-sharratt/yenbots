//! The browsers' errors (§18.1): every uncaught error, rejected promise and
//! failed call the framework catches is posted to `/api/v1/errors` and
//! written under the app's source, `yenbot/<app>`, with the page, the
//! account, the stack and the browser.
//!
//! Repeats are counted rather than flooding: an error is written the first
//! time it is seen, and again only once a minute has passed, saying how many
//! times it came in between.

use std::collections::HashMap;
use std::sync::Mutex;

use serde::Deserialize;

use super::{Level, Line, PLATFORM};

/// How long the repeats of an error are counted before it is written again.
pub const WINDOW_MS: i64 = 60_000;

/// What the framework posts.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Report {
    /// The app the page is, or `_site` for the platform's own pages.
    pub app: String,
    /// The page's address.
    pub page: String,
    pub message: String,
    #[serde(default)]
    pub stack: Option<String>,
    /// How many more times the page saw it since it last posted it.
    #[serde(default)]
    pub repeats: u64,
}

/// The errors seen within the last [`WINDOW_MS`], each with how many times it
/// has come since it was written.
#[derive(Default)]
pub struct Repeats {
    seen: Mutex<HashMap<String, (i64, u64)>>,
}

/// More distinct errors than this held at once drops the expired.
const HELD: usize = 1024;

impl Repeats {
    pub fn new() -> Repeats {
        Repeats::default()
    }

    /// An error keyed `key`, come `times` times at `now`: `Some(n)` when it
    /// is written, `n` being how many times it came unwritten before this;
    /// `None` when it is counted instead.
    pub fn note(&self, key: &str, now: i64, times: u64) -> Option<u64> {
        let mut seen = self.seen.lock().unwrap_or_else(|e| e.into_inner());
        if seen.len() > HELD {
            seen.retain(|_, (at, _)| now - *at < WINDOW_MS);
        }
        match seen.get_mut(key) {
            Some((at, n)) if now - *at < WINDOW_MS => {
                *n += times;
                None
            }
            Some((at, n)) => {
                let unwritten = *n + times - 1;
                (*at, *n) = (now, 0);
                Some(unwritten)
            }
            None => {
                seen.insert(key.to_string(), (now, 0));
                Some(times - 1)
            }
        }
    }
}

/// `s` cut to at most `max` characters.
fn cut(s: &str, max: usize) -> String {
    match s.char_indices().nth(max) {
        Some((at, _)) => format!("{}…", &s[..at]),
        None => s.to_string(),
    }
}

/// The source a report is written under: the app's, or the platform's for
/// its own pages.
pub fn source(app: &str) -> String {
    if app == "_site" {
        PLATFORM.to_string()
    } else {
        format!("{PLATFORM}/{app}")
    }
}

/// The line `report` is written as, from `account` on `browser`, `earlier`
/// unwritten repeats before it.
pub fn line(report: &Report, account: &str, browser: &str, earlier: u64, now: i64) -> Line {
    let mut message = format!(
        "{} — at {} — account {account} on {}",
        cut(&report.message, 500),
        cut(&report.page, 300),
        cut(browser, 200)
    );
    if earlier > 0 {
        message.push_str(&format!(" — and {earlier} more since it was last written"));
    }
    if let Some(stack) = report.stack.as_deref().filter(|s| !s.is_empty()) {
        message.push('\n');
        message.push_str(&cut(stack, 2000));
    }
    Line {
        at_ms: now,
        level: Level::Error,
        source: source(&report.app),
        target: "browser".into(),
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **An error is written once a minute at most, with the count of the
    /// times between.**
    #[test]
    fn repeats_are_counted_not_written() {
        let r = Repeats::new();
        assert_eq!(r.note("TypeError", 0, 1), Some(0));
        assert_eq!(r.note("TypeError", 10_000, 1), None);
        assert_eq!(r.note("TypeError", 20_000, 5), None);
        assert_eq!(r.note("Other", 20_000, 1), Some(0));
        assert_eq!(r.note("TypeError", WINDOW_MS + 1, 1), Some(6));
        assert_eq!(r.note("TypeError", WINDOW_MS + 2, 1), None);
    }

    #[test]
    fn a_report_is_written_as_one_line_with_its_stack() {
        let report = Report {
            app: "recipes".into(),
            page: "https://yenbot.com.au/a/recipes/r/7".into(),
            message: "TypeError: r.title is undefined".into(),
            stack: Some("at render (pages/recipe.js:12:5)".into()),
            repeats: 0,
        };
        let l = line(&report, "1093", "Safari 18 on iPhone", 3, 1_000);
        assert_eq!(l.source, "yenbot/recipes");
        assert_eq!((l.level, l.target.as_str()), (Level::Error, "browser"));
        assert_eq!(
            l.message,
            "TypeError: r.title is undefined — at https://yenbot.com.au/a/recipes/r/7 — \
             account 1093 on Safari 18 on iPhone — and 3 more since it was last written\n\
             at render (pages/recipe.js:12:5)"
        );
        let site = Report {
            app: "_site".into(),
            stack: None,
            ..report
        };
        let l = line(&site, "1093", "Firefox", 0, 1_000);
        assert_eq!(l.source, "yenbot");
        assert!(l.message.ends_with("account 1093 on Firefox"));
    }

    #[test]
    fn a_long_message_is_cut() {
        assert_eq!(cut("Phở bò", 3), "Phở…");
        assert_eq!(cut("Phở", 3), "Phở");
    }
}
