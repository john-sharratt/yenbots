//! Apps and accounts over time (`candle/docs/yenbot_app_host.md` §20):
//! which apps are used ([`usage`]), each account's settings for each
//! ([`settings`]), the notes sent from them ([`feedback`]), and what becomes
//! of an app renamed ([`rename`]), removed or archived ([`archive`]), or of
//! a file no record names ([`orphans`]).
//!
//! [`settle`] brings the store into step with the apps' folders each time
//! they are read: a rename is taken once, an app whose folder went is
//! archived, and one whose folder came back is restored.

pub mod archive;
pub mod feedback;
pub mod orphans;
pub mod rename;
pub mod settings;
pub mod tables;
pub mod usage;

use std::collections::BTreeSet;

use anyhow::Result;

use crate::app::App;
use archive::{archive as archive_app, folder_gone, reason_of, restore, with_data, Reason};

/// What [`settle`] did.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Settled {
    pub renamed: Vec<(String, String)>,
    pub archived: Vec<String>,
    pub restored: Vec<String>,
}

/// Bring the store into step with the apps' folders at `now`, and read the
/// apps again when anything moved.
pub fn settle(app: &App, now: i64) -> Result<Settled> {
    let mut done = Settled::default();
    for m in app.apps.all() {
        let Some(old) = &m.renamed_from else { continue };
        let mut conn = app.store.conn();
        if rename::done(&conn, old)? || !with_data(&conn)?.contains(old) {
            continue;
        }
        rename::take(&mut conn, &app.cfg.files_dir(), old, &m.id, now)?;
        tracing::info!(target: "yenbot::apps", from = %old, to = %m.id, "renamed: its records, files and jobs moved");
        done.renamed.push((old.clone(), m.id.clone()));
    }
    let present: BTreeSet<String> = app.apps.present().into_iter().collect();
    let conn = app.store.conn();
    for id in with_data(&conn)? {
        if present.contains(&id) {
            continue;
        }
        match reason_of(&conn, &id)? {
            None => {
                archive_app(&conn, &id, Reason::Removed, now)?;
                tracing::info!(target: "yenbot::apps", app = %id, "its folder is gone: archived, kept thirty days");
                done.archived.push(id);
            }
            Some(Reason::Admin) => folder_gone(&conn, &id, now)?,
            Some(Reason::Removed) => {}
        }
    }
    for id in &present {
        if reason_of(&conn, id)? == Some(Reason::Removed) {
            restore(&conn, id, now)?;
            tracing::info!(target: "yenbot::apps", app = %id, "its folder is back: restored");
            done.restored.push(id.clone());
        }
    }
    drop(conn);
    if done != Settled::default() {
        app.apps.reload();
    }
    Ok(done)
}

#[cfg(test)]
mod tests;
