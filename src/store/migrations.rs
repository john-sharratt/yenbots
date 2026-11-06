//! The platform's schema, as the ordered steps that build it. A store records
//! the last step it took in `schema_version`; opening it takes every step
//! after that, each in its own transaction, so a store from any earlier build
//! comes up to this one and a step half-taken is never left behind.
//!
//! A step is never edited once released: a change to the schema is a new
//! step at the end.

use anyhow::{bail, Context, Result};
use rusqlite::{params, Connection};

/// The steps, in order: step `n` is `STEPS[n - 1]`.
pub const STEPS: &[&str] = &[
    // 1 — accounts and the site's own log lines.
    r#"
    CREATE TABLE accounts (
      id          TEXT PRIMARY KEY,   -- the Google subject
      provider    TEXT NOT NULL,      -- the issuer half of the account key
      email       TEXT NOT NULL,
      name        TEXT NOT NULL,
      picture     TEXT,
      status      TEXT NOT NULL CHECK (status IN ('pending', 'active', 'blocked')),
      created_ms  INTEGER NOT NULL,
      seen_ms     INTEGER NOT NULL
    );
    CREATE INDEX accounts_email ON accounts (email);

    CREATE TABLE log_lines (
      id       INTEGER PRIMARY KEY,
      at_ms    INTEGER NOT NULL,
      level    TEXT NOT NULL,
      source   TEXT NOT NULL,       -- `yenbot`, or `yenbot/<app>`
      target   TEXT NOT NULL,
      message  TEXT NOT NULL
    );
    CREATE INDEX log_lines_source_at ON log_lines (source, at_ms);
    "#,
];

/// The schema this build makes.
pub const LATEST: u32 = STEPS.len() as u32;

/// The step the store last took; 0 for a new one.
pub fn version(conn: &Connection) -> Result<u32> {
    conn.execute_batch("CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL);")?;
    Ok(conn
        .query_row("SELECT MAX(version) FROM schema_version", [], |r| {
            r.get::<_, Option<u32>>(0)
        })?
        .unwrap_or(0))
}

/// Take every step after the store's.
pub fn migrate(conn: &mut Connection) -> Result<()> {
    let at = version(conn)?;
    if at > LATEST {
        bail!(
            "the store is at schema {at}, newer than this build's {LATEST}: run the build that \
             made it, or a newer one"
        );
    }
    for (i, step) in STEPS.iter().enumerate().skip(at as usize) {
        let n = i as u32 + 1;
        let tx = conn.transaction()?;
        tx.execute_batch(step)
            .with_context(|| format!("schema step {n}"))?;
        tx.execute(
            "INSERT INTO schema_version (version) VALUES (?1)",
            params![n],
        )?;
        tx.commit()?;
        tracing::info!(step = n, "store: schema step taken");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_store_takes_every_step_once() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn).unwrap();
        assert_eq!(version(&conn).unwrap(), LATEST);
        migrate(&mut conn).unwrap();
        let taken: u32 = conn
            .query_row("SELECT COUNT(*) FROM schema_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(taken, LATEST, "no step twice");
    }

    #[test]
    fn a_store_from_a_newer_build_is_refused() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn).unwrap();
        conn.execute(
            "INSERT INTO schema_version (version) VALUES (?1)",
            params![LATEST + 1],
        )
        .unwrap();
        assert!(migrate(&mut conn).is_err());
    }
}
