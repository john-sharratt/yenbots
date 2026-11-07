//! Which apps get used (§20.3): the framework posts an open once per page
//! load, counted per app, account and day in site time, and the admins read
//! the opens with what each app asked of the model and the image engine,
//! summed by day or by account.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use jiff::tz::TimeZone;
use jiff::Timestamp;
use rusqlite::{params, Connection};
use serde::Serialize;

/// The day `ms` falls on in `tz`, `YYYY-MM-DD`.
pub fn day_of(ms: i64, tz: &TimeZone) -> String {
    Timestamp::from_millisecond(ms)
        .map(|t| t.to_zoned(tz.clone()).date().to_string())
        .unwrap_or_default()
}

/// Count one open of `app` by `account` at `now`.
pub fn opened(conn: &Connection, app: &str, account: &str, now: i64, tz: &TimeZone) -> Result<()> {
    conn.execute(
        "INSERT INTO app_opens (app, account, day, opens) VALUES (?1, ?2, ?3, 1) \
         ON CONFLICT (app, account, day) DO UPDATE SET opens = opens + 1",
        params![app, account, day_of(now, tz)],
    )?;
    Ok(())
}

/// What the sums are grouped by beside the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum By {
    Day,
    Account,
}

impl By {
    pub fn parse(s: &str) -> Option<By> {
        match s {
            "day" => Some(By::Day),
            "account" => Some(By::Account),
            _ => None,
        }
    }
}

/// One app's use on one day, or by one account, since the start asked.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Used {
    pub app: String,
    /// The day, or the account's id.
    pub key: String,
    pub opens: i64,
    /// The accounts that opened it — one, when summed by account.
    pub accounts: i64,
    pub chats: i64,
    pub pictures: i64,
    pub questions: i64,
}

/// Opens and AI calls of `app` (every app when `None`) from day `since`
/// (`YYYY-MM-DD`, site time), by `by`, apps then keys in order.
pub fn summed(
    conn: &Connection,
    app: Option<&str>,
    since: &str,
    by: By,
    tz: &TimeZone,
) -> Result<Vec<Used>> {
    let mut rows: BTreeMap<(String, String), Used> = BTreeMap::new();
    let mut who: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    let mut stmt = conn.prepare(
        "SELECT app, account, day, opens FROM app_opens \
         WHERE day >= ?1 AND (?2 IS NULL OR app = ?2)",
    )?;
    let opens = stmt.query_map(params![since, app], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, i64>(3)?,
        ))
    })?;
    for o in opens {
        let (a, account, day, n) = o?;
        let key = match by {
            By::Day => day,
            By::Account => account.clone(),
        };
        let at = (a.clone(), key.clone());
        let row = rows.entry(at.clone()).or_insert_with(|| Used {
            app: a,
            key,
            ..Used::default()
        });
        row.opens += n;
        who.entry(at).or_default().insert(account);
    }
    let mut stmt = conn.prepare(
        "SELECT app, COALESCE(account, ''), kind, at_ms FROM ai_calls \
         WHERE (?1 IS NULL OR app = ?1)",
    )?;
    let calls = stmt.query_map(params![app], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, i64>(3)?,
        ))
    })?;
    for c in calls {
        let (a, account, kind, at_ms) = c?;
        let day = day_of(at_ms, tz);
        if day.as_str() < since {
            continue;
        }
        let key = match by {
            By::Day => day,
            By::Account => account,
        };
        let row = rows
            .entry((a.clone(), key.clone()))
            .or_insert_with(|| Used {
                app: a,
                key,
                ..Used::default()
            });
        match kind.as_str() {
            "chat" => row.chats += 1,
            "image" => row.pictures += 1,
            _ => row.questions += 1,
        }
    }
    Ok(rows
        .into_iter()
        .map(|(at, mut row)| {
            row.accounts = who.get(&at).map_or(0, |w| w.len() as i64);
            row
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;

    const DAY: i64 = 86_400_000;

    /// **Opens are counted per account and day, and summed with the AI
    /// calls by day or by account, from the day asked.**
    #[test]
    fn opens_and_calls_are_summed() {
        let store = Store::in_memory().unwrap();
        let conn = store.conn();
        let tz = TimeZone::UTC;
        for (account, at) in [("1", 0), ("1", 5), ("2", 9), ("1", DAY + 1)] {
            opened(&conn, "recipes", account, at, &tz).unwrap();
        }
        opened(&conn, "chores", "2", DAY, &tz).unwrap();
        for (kind, account, at) in [
            ("chat", Some("1"), 3),
            ("image", Some("1"), DAY),
            ("ask", None, DAY),
        ] {
            conn.execute(
                "INSERT INTO ai_calls (app, account, kind, asked, ms, outcome, at_ms) \
                 VALUES ('recipes', ?1, ?2, 'q', 1, 'ok', ?3)",
                params![account, kind, at],
            )
            .unwrap();
        }
        let by_day = summed(&conn, Some("recipes"), "1970-01-01", By::Day, &tz).unwrap();
        assert_eq!(
            by_day,
            [
                Used {
                    app: "recipes".into(),
                    key: "1970-01-01".into(),
                    opens: 3,
                    accounts: 2,
                    chats: 1,
                    ..Used::default()
                },
                Used {
                    app: "recipes".into(),
                    key: "1970-01-02".into(),
                    opens: 1,
                    accounts: 1,
                    pictures: 1,
                    questions: 1,
                    ..Used::default()
                },
            ]
        );
        let by_account = summed(&conn, None, "1970-01-02", By::Account, &tz).unwrap();
        let shown: Vec<_> = by_account
            .iter()
            .map(|u| {
                (
                    u.app.as_str(),
                    u.key.as_str(),
                    u.opens,
                    u.pictures,
                    u.questions,
                )
            })
            .collect();
        assert_eq!(
            shown,
            [
                ("chores", "2", 1, 0, 0),
                ("recipes", "", 0, 0, 1),
                ("recipes", "1", 1, 1, 0)
            ]
        );
        assert_eq!(
            day_of(DAY - 1, &TimeZone::get("Australia/Sydney").unwrap()),
            "1970-01-02"
        );
    }
}
