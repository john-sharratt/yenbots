//! Collections taken to a new shape (`candle/docs/yenbot_app_host.md`
//! §7.5): when an app raises a collection's `version`, it ships
//! `workflows/migrate-<coll>-<n>.js`, and its records are taken up in the
//! background ([`migrate`]) while the app is served its last good version
//! ([`crate::apps`]). Once none is behind, the site serves the app as it
//! now is and its open pages reload.
//!
//! The worker also takes up a record a served app wrote at an older shape
//! in the moment before its new version was served, at its next look.

pub mod migrate;

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use rusqlite::{params, Connection};
use serde::Serialize;

use crate::accounts::now_ms;
use crate::app::App;
use crate::data::changes::Feed;
use crate::jobs::runner::env_of;
use crate::jobs::sync::sync;
use crate::logs::browser::source;
use crate::logs::{Level, Line};
use crate::workflows::sources;
use migrate::{behind, migrate};

/// How often the worker looks for records behind their app's version.
const LOOK: Duration = Duration::from_secs(60);

/// A collection's migration, as the admin page shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Reshape {
    pub coll: String,
    pub shape: u32,
    pub done: u64,
    pub total: u64,
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub updated_ms: i64,
}

/// App `app`'s migrations, by collection.
pub fn of_app(conn: &Connection, app: &str) -> Result<Vec<Reshape>> {
    let mut stmt = conn.prepare(
        "SELECT coll, shape, done, total, state, error, updated_ms FROM reshapes WHERE app = ?1 ORDER BY coll",
    )?;
    let rows = stmt.query_map(params![app], |r| {
        Ok(Reshape {
            coll: r.get(0)?,
            shape: r.get(1)?,
            done: r.get(2)?,
            total: r.get(3)?,
            state: r.get(4)?,
            error: r.get(5)?,
            updated_ms: r.get(6)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Migrate what is behind; answers whether an app waiting for its
/// migration is now ready to be served as it is. `failed` holds the apps,
/// with the commit they were read at, whose migration failed: one is tried
/// again at the next commit, not at the same one.
pub fn work(app: &App, failed: &Mutex<BTreeSet<(String, String)>>) -> bool {
    let env = env_of(app);
    let now = now_ms();
    let mut ready = false;
    for m in app.apps.pending() {
        let at = app
            .apps
            .checked(&m.id)
            .and_then(|c| c.commit)
            .unwrap_or_default();
        let key = (m.id.clone(), at);
        if failed
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains(&key)
        {
            continue;
        }
        let done = sources::in_tree(app.apps.root(), &m.id).and_then(|src| {
            // Read whole before migrating: a migration takes the store itself.
            let colls = behind(&env.store.conn(), &m)?;
            for coll in colls {
                let n = migrate(&env, &m, &src, &coll, now)?;
                tracing::info!(target: "yenbot::reshape", app = %m.id, coll = %coll, records = n, "migrated");
            }
            Ok(())
        });
        match done {
            Ok(()) => ready = true,
            Err(e) => {
                failed.lock().unwrap_or_else(|e| e.into_inner()).insert(key);
                let said = format!(
                    "the migration failed, and the app's last good version is served: {e:#}"
                );
                tracing::error!(target: "yenbot::reshape", app = %m.id, "{said}");
                let _ = app.logs.write(&[Line {
                    at_ms: now_ms(),
                    level: Level::Error,
                    source: source(&m.id),
                    target: "migration".into(),
                    message: said,
                }]);
            }
        }
    }
    let waiting: BTreeSet<String> = app.apps.pending().iter().map(|m| m.id.clone()).collect();
    for m in app.apps.all() {
        if waiting.contains(&m.id) {
            continue;
        }
        let late = sources::of(&app.apps, &m.id).and_then(|src| {
            let colls = behind(&env.store.conn(), &m)?;
            let any = !colls.is_empty();
            for coll in colls {
                migrate(&env, &m, &src, &coll, now)?;
            }
            Ok(any)
        });
        match late {
            Ok(false) => {}
            // What was taken up keeps its jobs in step — a moved time moves
            // its reminder — and the app's open pages read it again.
            Ok(true) => {
                if let Err(e) = sync(&mut app.store.conn(), &m, &app.cfg.zone(), now) {
                    tracing::error!(target: "yenbot::reshape", app = %m.id, "its jobs could not be brought into step: {e:#}");
                }
                app.changes.send(Feed::Reset(Some(m.id.clone())));
            }
            Err(e) => {
                tracing::error!(target: "yenbot::reshape", app = %m.id, "a record written at an older shape could not be migrated: {e:#}");
            }
        }
    }
    ready
}

#[cfg(test)]
#[path = "work_tests.rs"]
mod tests;

/// Start the worker: it migrates whenever an app waits for it, and looks
/// for records left behind every minute. An app made ready is served as it
/// is on the reload that follows, which every open page hears.
pub fn start(app: Arc<App>) {
    tokio::spawn(async move {
        let failed = Arc::new(Mutex::new(BTreeSet::new()));
        let wanted = app.apps.migrations();
        loop {
            tokio::select! {
                _ = wanted.notified() => {}
                _ = tokio::time::sleep(LOOK) => {}
            }
            let (worker, held) = (app.clone(), failed.clone());
            let ready = tokio::task::spawn_blocking(move || work(&worker, &held)).await;
            if let Ok(true) = ready {
                app.reloads.moved(&app.apps.head().unwrap_or_default());
            }
        }
    });
}
