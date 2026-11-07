//! The jobs a write makes, moves or ends, in the write's own transaction
//! (§9.2):
//!
//! - a `from:` schedule keeps one job per record, at the record's time less
//!   its `before:` — made with the record, moved with its time, gone with
//!   it; a record whose time has passed, or that has none, has no job
//!   waiting;
//! - a `when:` schedule makes a job, due at once, for each write it hears,
//!   carrying the record before and after.
//!
//! A record's job acts for the record's owner; a write's job for whoever
//! wrote it. Either acts as the app itself when that was the app.

use anyhow::Result;
use rusqlite::{params, Connection};
use serde_json::{json, Value};

use super::table::{insert, of_source, NewJob, State};
use crate::accounts::caller::{Caller, APP_PREFIX};
use crate::apps::manifest::Manifest;
use crate::apps::schedule::{Does, Kind};
use crate::data::docs::Doc;

/// The source of schedule `n`'s job for record `id` of `coll`.
pub fn record_source(n: usize, coll: &str, id: &str) -> String {
    format!("record:{n}:{coll}/{id}")
}

/// The account a job acts for, when it is one: an app acting as itself
/// has none, and its job acts as the app again.
pub fn acting(id: &str) -> Option<&str> {
    (!id.starts_with(APP_PREFIX)).then_some(id)
}

/// When record `doc`'s job of a `from:` schedule is due: its time field
/// `at` less `before_ms`, if that is still to come.
pub fn due_of(doc: &Doc, at: &str, before_ms: i64, now: i64) -> Option<i64> {
    let time = doc.data.get(at)?.as_i64()?;
    let due = time - before_ms;
    (due > now).then_some(due)
}

/// Keep schedule `n`'s job for `doc` at `due`: made, moved, or — with no
/// `due` — ended unless it already ran. A held job keeps its hold.
pub fn keep_record_job(
    conn: &Connection,
    m: &Manifest,
    n: usize,
    coll: &str,
    doc: &Doc,
    due: Option<i64>,
    now: i64,
) -> Result<()> {
    let run = &m.schedules[n].run;
    let source = record_source(n, coll, &doc.id);
    let held = of_source(conn, &m.id, &source)?;
    let Some(due) = due else {
        conn.execute(
            "DELETE FROM jobs WHERE app = ?1 AND source = ?2 AND state IN ('waiting', 'held')",
            params![m.id, source],
        )?;
        return Ok(());
    };
    match held.first() {
        Some(j) if j.next_ms == Some(due) && &j.workflow == run && j.state != State::Done => {}
        Some(j) => {
            conn.execute(
                "UPDATE jobs SET next_ms = ?2, workflow = ?3, attempts = 0, run_as = ?4, \
                 state = CASE state WHEN 'held' THEN 'held' ELSE 'waiting' END, updated_ms = ?5 \
                 WHERE id = ?1",
                params![j.id, due, run, acting(&doc.owner), now],
            )?;
        }
        None => {
            insert(
                conn,
                &NewJob {
                    app: &m.id,
                    workflow: run,
                    args: &json!({"coll": coll, "id": doc.id}),
                    source: &source,
                    rule: None,
                    catch_up: false,
                    next_ms: due,
                    run_as: acting(&doc.owner),
                },
                now,
            )?;
        }
    }
    Ok(())
}

/// Keep app `m`'s jobs in step with a write to `coll`: `before` is the
/// record as it was (`None` for a create), `after` as it is (`None` for a
/// delete), written by `by`.
pub fn written(
    conn: &Connection,
    m: &Manifest,
    coll: &str,
    before: Option<&Doc>,
    after: Option<&Doc>,
    by: &Caller,
    now: i64,
) -> Result<()> {
    for (n, s) in m.schedules.iter().enumerate() {
        // The manifest was read whole when it was served, so each kind reads.
        let Ok(kind) = s.kind(&m.collections) else {
            continue;
        };
        match kind {
            Kind::Every(_) => {}
            Kind::From {
                coll: c,
                at,
                before_ms,
            } if c == coll => match (before, after) {
                (_, Some(doc)) => {
                    keep_record_job(conn, m, n, coll, doc, due_of(doc, at, before_ms, now), now)?
                }
                (Some(doc), None) => {
                    conn.execute(
                        "DELETE FROM jobs WHERE app = ?1 AND source = ?2",
                        params![m.id, record_source(n, coll, &doc.id)],
                    )?;
                }
                (None, None) => {}
            },
            Kind::From { .. } => {}
            Kind::When(w) if w.collection == coll => {
                let heard = match (w.does, before, after) {
                    (Does::Create, None, Some(a)) => w.matches(&a.data),
                    (Does::Update, Some(b), Some(a)) => w.matches(&a.data) && !w.matches(&b.data),
                    (Does::Delete, Some(b), None) => w.matches(&b.data),
                    _ => false,
                };
                if !heard {
                    continue;
                }
                let id = after.or(before).map(|d| d.id.as_str()).unwrap_or_default();
                let args = json!({
                    "before": before.map_or(Value::Null, Doc::record),
                    "after": after.map_or(Value::Null, Doc::record),
                });
                insert(
                    conn,
                    &NewJob {
                        app: &m.id,
                        workflow: &s.run,
                        args: &args,
                        source: &format!("write:{coll}/{id}"),
                        rule: None,
                        catch_up: false,
                        next_ms: now,
                        run_as: acting(by.id()),
                    },
                    now,
                )?;
            }
            Kind::When(_) => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounts::{Account, Status};
    use crate::data::docs::{create, delete, update, Change};
    use crate::jobs::table::{list, Job};
    use crate::store::Store;

    const APP: &str = "id: home\nname: Home\ncollections:\n  reminders:\n    scope: private\n    fields:\n      \
                       title: { type: text }\n      due_ms: { type: time }\n  chores:\n    fields:\n      \
                       done: { type: bool }\nschedules:\n  - from: reminders\n    at: due_ms\n    before: 10m\n    \
                       run: remind\n  - when: {collection: chores, does: update, where: {done: true}}\n    run: chore-done\n";

    fn yen() -> Caller {
        Caller {
            account: Account {
                id: "1".into(),
                email: "yen@example.com".into(),
                name: "Yen".into(),
                picture: None,
                status: Status::Active,
                admin: true,
                created_ms: 0,
                seen_ms: 0,
            },
            groups: Vec::new(),
            token: None,
        }
    }

    fn jobs(store: &Store) -> Vec<Job> {
        list(&store.conn(), Some("home"), None).unwrap()
    }

    const MIN: i64 = 60_000;

    /// **A reminder's job is made with it, moves with its time, and goes
    /// with it**; one whose time has passed has none.
    #[test]
    fn a_records_job_follows_the_record() {
        let store = Store::in_memory().unwrap();
        let m = Manifest::parse(APP, "home").unwrap();
        let now = 1_000 * MIN;
        let mut conn = store.conn();
        let tx = conn.transaction().unwrap();
        let r = create(
            &tx,
            &m,
            "reminders",
            &yen(),
            &json!({"title": "Call Mai", "due_ms": now + 60 * MIN}),
            now,
        )
        .unwrap()
        .unwrap();
        let past = create(
            &tx,
            &m,
            "reminders",
            &yen(),
            &json!({"title": "Gone", "due_ms": now - MIN}),
            now,
        )
        .unwrap()
        .unwrap();
        tx.commit().unwrap();
        drop(conn);
        let made = jobs(&store);
        assert_eq!(made.len(), 1, "a time passed has no job");
        assert_eq!(made[0].source, format!("record:0:reminders/{}", r.id));
        assert_eq!(made[0].next_ms, Some(now + 50 * MIN), "ten minutes before");
        assert_eq!(made[0].workflow, "remind");
        assert_eq!(made[0].run_as.as_deref(), Some("1"));
        assert_eq!(made[0].args, json!({"coll": "reminders", "id": r.id}));

        let mut conn = store.conn();
        let tx = conn.transaction().unwrap();
        let later = json!({"due_ms": now + 120 * MIN});
        update(
            &tx,
            &m,
            "reminders",
            &yen(),
            &r.id,
            Change {
                patch: &later,
                expect: None,
            },
            now,
        )
        .unwrap()
        .unwrap();
        let soon = json!({"due_ms": now + 30 * MIN});
        update(
            &tx,
            &m,
            "reminders",
            &yen(),
            &past.id,
            Change {
                patch: &soon,
                expect: None,
            },
            now,
        )
        .unwrap()
        .unwrap();
        tx.commit().unwrap();
        drop(conn);
        let moved = jobs(&store);
        assert_eq!(moved.len(), 2);
        assert_eq!(
            moved[0].next_ms,
            Some(now + 20 * MIN),
            "the one now to come"
        );
        assert_eq!(
            moved[1].next_ms,
            Some(now + 110 * MIN),
            "the one moved later"
        );
        assert_eq!(moved[1].id, made[0].id, "moved, not made again");

        let mut conn = store.conn();
        let tx = conn.transaction().unwrap();
        delete(&tx, &m, "reminders", &yen(), &r.id, now)
            .unwrap()
            .unwrap();
        tx.commit().unwrap();
        drop(conn);
        assert_eq!(jobs(&store).len(), 1, "gone with its record");
    }

    /// **A chore coming to be done makes a job with the record before and
    /// after; an edit to a chore already done does not.**
    #[test]
    fn a_write_a_when_hears_makes_a_job() {
        let store = Store::in_memory().unwrap();
        let m = Manifest::parse(APP, "home").unwrap();
        let mut conn = store.conn();
        let tx = conn.transaction().unwrap();
        let c = create(&tx, &m, "chores", &yen(), &json!({"done": false}), 5)
            .unwrap()
            .unwrap();
        let done = json!({"done": true});
        update(
            &tx,
            &m,
            "chores",
            &yen(),
            &c.id,
            Change {
                patch: &done,
                expect: None,
            },
            6,
        )
        .unwrap()
        .unwrap();
        update(
            &tx,
            &m,
            "chores",
            &yen(),
            &c.id,
            Change {
                patch: &done,
                expect: None,
            },
            7,
        )
        .unwrap()
        .unwrap();
        tx.commit().unwrap();
        drop(conn);
        let made = jobs(&store);
        assert_eq!(made.len(), 1, "once, as it came to be done");
        let j = &made[0];
        assert_eq!((j.workflow.as_str(), j.next_ms), ("chore-done", Some(6)));
        assert_eq!(j.source, format!("write:chores/{}", c.id));
        assert_eq!(j.args["before"]["done"], false);
        assert_eq!(j.args["after"]["done"], true);
        assert_eq!(j.args["after"]["version"], 2);
        assert_eq!(j.args["after"]["id"], json!(c.id));
    }

    /// **A record the app itself made has a job that acts as the app.**
    #[test]
    fn an_apps_own_record_acts_as_the_app() {
        let store = Store::in_memory().unwrap();
        let m = Manifest::parse(APP, "home").unwrap();
        let mut conn = store.conn();
        let tx = conn.transaction().unwrap();
        let app = Caller::app("home", "Home");
        create(&tx, &m, "reminders", &app, &json!({"due_ms": 99 * MIN}), 0)
            .unwrap()
            .unwrap();
        tx.commit().unwrap();
        drop(conn);
        assert_eq!(jobs(&store)[0].run_as, None);
    }
}
