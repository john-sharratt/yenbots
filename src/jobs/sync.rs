//! An app's jobs brought into step with its manifest each time it is
//! served (§5.3, §9.2): one job per repeat rule, and one per record of each
//! `from:` schedule. A rule that is unchanged keeps its job and its time; a
//! changed one is due at its new next time; one gone takes its job with it.
//! Jobs made by writes and workflows are theirs, and are left alone.
//!
//! A record's job whose time passed while the site was down is left for the
//! scheduler, which runs it if it is recent (§9.4); only a write ends it.

use std::collections::BTreeSet;

use anyhow::Result;
use jiff::tz::TimeZone;
use rusqlite::{params, Connection};
use serde_json::json;

use super::follow::{due_of, keep_record_job, record_source};
use super::rule::Rule;
use super::table::{insert, of_source, NewJob};
use crate::apps::manifest::Manifest;
use crate::apps::schedule::Kind;
use crate::data::docs::all_of;

/// The source of repeat rule `n`'s job.
pub fn schedule_source(n: usize) -> String {
    format!("schedule:{n}")
}

/// Bring app `m`'s jobs into step with it, at `now` in the zone `tz`.
pub fn sync(conn: &mut Connection, m: &Manifest, tz: &TimeZone, now: i64) -> Result<()> {
    let tx = conn.transaction()?;
    let mut repeats = BTreeSet::new();
    let mut record_prefixes = Vec::new();
    for (n, s) in m.schedules.iter().enumerate() {
        let Ok(kind) = s.kind(&m.collections) else {
            continue;
        };
        match kind {
            Kind::Every(rule) => {
                let source = schedule_source(n);
                repeats.insert(source.clone());
                keep_repeat(&tx, m, n, &source, &rule, tz, now)?;
            }
            Kind::From {
                coll,
                at,
                before_ms,
            } => {
                record_prefixes.push(format!("record:{n}:{coll}/"));
                let docs = all_of(&tx, &m.id, coll)?;
                let ids: BTreeSet<String> =
                    docs.iter().map(|d| record_source(n, coll, &d.id)).collect();
                for doc in &docs {
                    if let Some(due) = due_of(doc, at, before_ms, now) {
                        keep_record_job(&tx, m, n, coll, doc, Some(due), now)?;
                    }
                }
                for j in of_source(&tx, &m.id, &format!("record:{n}:"))? {
                    if !ids.contains(&j.source) {
                        tx.execute("DELETE FROM jobs WHERE id = ?1", params![j.id])?;
                    }
                }
            }
            Kind::When(_) => {}
        }
    }
    for j in of_source(&tx, &m.id, "schedule:")? {
        if !repeats.contains(&j.source) {
            tx.execute("DELETE FROM jobs WHERE id = ?1", params![j.id])?;
        }
    }
    for j in of_source(&tx, &m.id, "record:")? {
        let ours = record_prefixes.iter().any(|p| j.source.starts_with(p));
        if !ours {
            tx.execute("DELETE FROM jobs WHERE id = ?1", params![j.id])?;
        }
    }
    tx.commit()?;
    Ok(())
}

/// Keep repeat rule `n`'s job, due at its next time.
fn keep_repeat(
    conn: &Connection,
    m: &Manifest,
    n: usize,
    source: &str,
    rule: &Rule,
    tz: &TimeZone,
    now: i64,
) -> Result<()> {
    let s = &m.schedules[n];
    let every = s
        .every
        .as_deref()
        .expect("a repeat rule's schedule has `every:`");
    let next = rule.next(now, tz);
    let held = of_source(conn, &m.id, source)?;
    match held.first() {
        Some(j) if j.workflow == s.run && j.rule.as_deref() == Some(every) => {
            if j.catch_up != s.catch_up {
                conn.execute(
                    "UPDATE jobs SET catch_up = ?2, updated_ms = ?3 WHERE id = ?1",
                    params![j.id, s.catch_up, now],
                )?;
            }
        }
        Some(j) => {
            conn.execute(
                "UPDATE jobs SET workflow = ?2, rule = ?3, catch_up = ?4, next_ms = ?5, attempts = 0, \
                 state = CASE state WHEN 'held' THEN 'held' ELSE 'waiting' END, updated_ms = ?6 WHERE id = ?1",
                params![j.id, s.run, every, s.catch_up, next, now],
            )?;
        }
        None => {
            if let Some(next) = next {
                insert(
                    conn,
                    &NewJob {
                        app: &m.id,
                        workflow: &s.run,
                        args: &json!({}),
                        source,
                        rule: Some(every),
                        catch_up: s.catch_up,
                        next_ms: next,
                        run_as: None,
                    },
                    now,
                )?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;
    use crate::jobs::table::{list, Job};
    use crate::store::Store;

    fn sydney() -> TimeZone {
        TimeZone::get("Australia/Sydney").unwrap()
    }

    fn at(d: i8, h: i8, mi: i8) -> i64 {
        sydney()
            .to_ambiguous_zoned(date(2026, 11, d).at(h, mi, 0, 0))
            .compatible()
            .unwrap()
            .timestamp()
            .as_millisecond()
    }

    fn manifest(schedules: &str) -> Manifest {
        Manifest::parse(
            &format!(
                "id: home\nname: Home\ncollections:\n  reminders:\n    fields:\n      due_ms: {{ type: time }}\n\
                 schedules:\n{schedules}"
            ),
            "home",
        )
        .unwrap()
    }

    fn jobs(store: &Store) -> Vec<Job> {
        list(&store.conn(), Some("home"), None).unwrap()
    }

    fn put(store: &Store, id: &str, due: i64) {
        store
            .conn()
            .execute(
                "INSERT INTO docs (app, coll, id, owner, data, version, created_ms, updated_ms) \
                 VALUES ('home', 'reminders', ?1, '1', ?2, 1, 0, 0)",
                params![id, json!({"due_ms": due}).to_string()],
            )
            .unwrap();
    }

    /// **A repeat rule has one job, kept at its time while unchanged, moved
    /// when it changes, gone when it goes.**
    #[test]
    fn a_rule_keeps_one_job() {
        let store = Store::in_memory().unwrap();
        let now = at(7, 10, 15);
        let weekly = manifest("  - every: weekly sun 19:00\n    run: plan-meals\n");
        sync(&mut store.conn(), &weekly, &sydney(), now).unwrap();
        let first = jobs(&store);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].source, "schedule:0");
        assert_eq!(first[0].next_ms, Some(at(8, 19, 0)));
        assert_eq!(first[0].rule.as_deref(), Some("weekly sun 19:00"));

        sync(&mut store.conn(), &weekly, &sydney(), now + 1_000).unwrap();
        assert_eq!(jobs(&store), first, "unchanged");

        let daily = manifest("  - every: daily 07:30\n    run: plan-meals\n    catch_up: true\n");
        sync(&mut store.conn(), &daily, &sydney(), now).unwrap();
        let moved = jobs(&store);
        assert_eq!(moved[0].id, first[0].id);
        assert_eq!(moved[0].next_ms, Some(at(8, 7, 30)));
        assert!(moved[0].catch_up);

        sync(&mut store.conn(), &manifest("  []\n"), &sydney(), now).unwrap();
        assert!(jobs(&store).is_empty());
    }

    /// **Each record of a `from:` has its job, made for records written while
    /// the schedule was not there; one whose record is gone goes, and one
    /// whose time passed is left for the scheduler.**
    #[test]
    fn a_from_keeps_a_job_per_record() {
        let store = Store::in_memory().unwrap();
        let now = at(7, 10, 0);
        put(&store, "r1", now + 3_600_000);
        put(&store, "r2", now + 7_200_000);
        let from = manifest("  - from: reminders\n    at: due_ms\n    run: remind\n");
        sync(&mut store.conn(), &from, &sydney(), now).unwrap();
        let made = jobs(&store);
        assert_eq!(
            made.iter()
                .map(|j| (j.source.as_str(), j.next_ms))
                .collect::<Vec<_>>(),
            [
                ("record:0:reminders/r1", Some(now + 3_600_000)),
                ("record:0:reminders/r2", Some(now + 7_200_000))
            ]
        );
        store
            .conn()
            .execute("DELETE FROM docs WHERE id = 'r2'", [])
            .unwrap();
        sync(&mut store.conn(), &from, &sydney(), now + 7_000_000).unwrap();
        let left = jobs(&store);
        assert_eq!(left.len(), 1, "r2's record is gone");
        assert_eq!(
            left[0].next_ms,
            Some(now + 3_600_000),
            "r1's passed, and is kept"
        );

        // Run early by an admin, a job is not put back by a reload before it runs.
        put(&store, "r3", now + 20_000_000);
        sync(&mut store.conn(), &from, &sydney(), now).unwrap();
        store
            .conn()
            .execute(
                "UPDATE jobs SET next_ms = ?1 WHERE source = 'record:0:reminders/r3'",
                [now + 5_000_000],
            )
            .unwrap();
        sync(&mut store.conn(), &from, &sydney(), now + 6_000_000).unwrap();
        let r3 = jobs(&store)
            .into_iter()
            .find(|j| j.source == "record:0:reminders/r3")
            .unwrap();
        assert_eq!(r3.next_ms, Some(now + 5_000_000));
        store
            .conn()
            .execute("DELETE FROM docs WHERE id = 'r3'", [])
            .unwrap();

        sync(&mut store.conn(), &manifest("  []\n"), &sydney(), now).unwrap();
        assert!(jobs(&store).is_empty(), "the schedule is gone");
    }
}
