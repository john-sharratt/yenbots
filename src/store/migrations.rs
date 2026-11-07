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
    // 2 — the apps' documents, their search index, groups, shares, app
    // access and API tokens.
    r#"
    CREATE TABLE docs (
      app        TEXT NOT NULL,
      coll       TEXT NOT NULL,
      id         TEXT NOT NULL,
      owner      TEXT NOT NULL,
      data       TEXT NOT NULL,
      version    INTEGER NOT NULL,
      created_ms INTEGER NOT NULL,
      updated_ms INTEGER NOT NULL,
      PRIMARY KEY (app, coll, id)
    );
    CREATE INDEX docs_by_owner ON docs (app, coll, owner, updated_ms);

    CREATE VIRTUAL TABLE docs_fts USING fts5(
      app UNINDEXED, coll UNINDEXED, id UNINDEXED, body,
      tokenize = 'unicode61 remove_diacritics 2'
    );

    CREATE TABLE groups (
      name        TEXT PRIMARY KEY,
      created_ms  INTEGER NOT NULL
    );
    CREATE TABLE group_members (
      grp      TEXT NOT NULL REFERENCES groups (name) ON DELETE CASCADE,
      account  TEXT NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
      PRIMARY KEY (grp, account)
    );

    -- One record shared with an account or a group, to read or to edit.
    CREATE TABLE shares (
      app    TEXT NOT NULL,
      coll   TEXT NOT NULL,
      id     TEXT NOT NULL,
      who    TEXT NOT NULL,           -- 'account:<id>' or 'group:<name>'
      edit   INTEGER NOT NULL,
      PRIMARY KEY (app, coll, id, who)
    );

    -- An app's visibility and who a private one is shared with (§12.4).
    CREATE TABLE app_access (
      app      TEXT PRIMARY KEY,
      private  INTEGER NOT NULL DEFAULT 0,
      position INTEGER
    );
    CREATE TABLE app_shares (
      app  TEXT NOT NULL,
      who  TEXT NOT NULL,             -- 'account:<id>' or 'group:<name>'
      PRIMARY KEY (app, who)
    );

    -- The site's own settings, changed on the admin page: `registration`.
    CREATE TABLE site_settings (
      key    TEXT PRIMARY KEY,
      value  TEXT NOT NULL
    );

    -- API tokens (§6.3): only the hash is kept.
    CREATE TABLE api_tokens (
      hash        TEXT PRIMARY KEY,
      name        TEXT NOT NULL UNIQUE,
      account     TEXT NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
      created_ms  INTEGER NOT NULL,
      expires_ms  INTEGER NOT NULL
    );
    "#,
    // 3 — jobs and their runs, what a workflow did once, and the shape each
    // record is at.
    r#"
    CREATE TABLE jobs (
      id          TEXT PRIMARY KEY,
      app         TEXT NOT NULL,      -- '_platform' for the platform's own
      workflow    TEXT NOT NULL,
      args        TEXT NOT NULL,      -- JSON
      source      TEXT NOT NULL,      -- 'schedule:<n>' | 'record:<n>:<coll>/<id>' | 'write:<coll>/<id>' | 'workflow'
      rule        TEXT,               -- a repeating job's rule
      catch_up    INTEGER NOT NULL DEFAULT 0,
      next_ms     INTEGER,            -- when it is due; NULL: not again
      attempts    INTEGER NOT NULL DEFAULT 0,
      state       TEXT NOT NULL CHECK (state IN ('waiting', 'running', 'held', 'done', 'failed')),
      run_as      TEXT,               -- the account it acts for; the app's own when NULL
      updated_ms  INTEGER NOT NULL
    );
    CREATE INDEX jobs_due ON jobs (next_ms) WHERE state = 'waiting';
    CREATE INDEX jobs_by_app ON jobs (app, source);

    CREATE TABLE job_runs (
      id          INTEGER PRIMARY KEY,
      job         TEXT,               -- NULL for a workflow run at once
      app         TEXT NOT NULL,
      workflow    TEXT NOT NULL,
      started_ms  INTEGER NOT NULL,
      ended_ms    INTEGER NOT NULL,
      outcome     TEXT NOT NULL CHECK (outcome IN ('ok', 'failed')),
      error       TEXT,
      log         TEXT NOT NULL
    );
    CREATE INDEX job_runs_by_job ON job_runs (job, started_ms);
    CREATE INDEX job_runs_by_app ON job_runs (app, started_ms);

    -- What `ctx.once(key, fn)` answered, so a run repeated does not do it again.
    CREATE TABLE job_once (
      app       TEXT NOT NULL,
      key       TEXT NOT NULL,
      value     TEXT NOT NULL,        -- JSON
      at_ms     INTEGER NOT NULL,
      PRIMARY KEY (app, key)
    );

    -- The version of its collection's shape a record was written at (§7.5).
    ALTER TABLE docs ADD COLUMN shape INTEGER NOT NULL DEFAULT 1;

    -- A collection's migration to a new shape, and how far it has come.
    CREATE TABLE reshapes (
      app         TEXT NOT NULL,
      coll        TEXT NOT NULL,
      shape       INTEGER NOT NULL,   -- the shape it is taking the records to
      done        INTEGER NOT NULL,
      total       INTEGER NOT NULL,
      state       TEXT NOT NULL CHECK (state IN ('running', 'done', 'failed')),
      error       TEXT,
      updated_ms  INTEGER NOT NULL,
      PRIMARY KEY (app, coll)
    );
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
