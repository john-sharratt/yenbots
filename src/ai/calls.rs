//! What each app asked the AI and the image engine (§14, §20.3): every call
//! kept in `ai_calls` — by which app and account, what was asked, how long it
//! took, how many tokens, how it ended — so the admin page shows what each
//! uses. There is no budget; this is what keeps it seen.

use anyhow::Result;
use rusqlite::{params, Connection};

/// The most of what was asked a call keeps, in characters.
const ASKED: usize = 500;

/// One call, as it is kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Call<'a> {
    pub app: &'a str,
    /// `None` for a workflow running as the app.
    pub account: Option<&'a str>,
    /// `chat`, `image` or `ask`.
    pub kind: &'static str,
    pub asked: &'a str,
    pub ms: i64,
    pub tokens_in: Option<u64>,
    pub tokens_out: Option<u64>,
    /// Why it failed, when it did.
    pub error: Option<&'a str>,
    pub at_ms: i64,
}

pub fn record(conn: &Connection, c: &Call<'_>) -> Result<()> {
    let asked: String = c.asked.chars().take(ASKED).collect();
    conn.execute(
        "INSERT INTO ai_calls (app, account, kind, asked, ms, tokens_in, tokens_out, outcome, error, at_ms) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            c.app,
            c.account,
            c.kind,
            asked,
            c.ms,
            c.tokens_in,
            c.tokens_out,
            if c.error.is_none() { "ok" } else { "failed" },
            c.error,
            c.at_ms
        ],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;

    #[test]
    fn a_call_is_kept_with_how_it_ended() {
        let store = Store::in_memory().unwrap();
        let conn = store.conn();
        let long = "x".repeat(900);
        record(
            &conn,
            &Call {
                app: "chores",
                account: Some("1"),
                kind: "chat",
                asked: &long,
                ms: 900,
                tokens_in: Some(40),
                tokens_out: Some(6),
                error: None,
                at_ms: 5,
            },
        )
        .unwrap();
        record(
            &conn,
            &Call {
                app: "chores",
                account: None,
                kind: "image",
                asked: "a teapot",
                ms: 40_000,
                tokens_in: None,
                tokens_out: None,
                error: Some("no room"),
                at_ms: 6,
            },
        )
        .unwrap();
        let rows: Vec<(String, Option<String>, i64, String)> = conn
            .prepare("SELECT kind, account, length(asked), outcome FROM ai_calls ORDER BY id")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(
            rows,
            [
                (
                    "chat".to_string(),
                    Some("1".to_string()),
                    500,
                    "ok".to_string()
                ),
                ("image".to_string(), None, 8, "failed".to_string())
            ]
        );
    }
}
