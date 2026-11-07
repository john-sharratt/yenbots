//! The `jobs` and `job_runs` tables (§9.1): a job is a row — which app,
//! which workflow, with what, when it is next due and how its last runs
//! went — and each run is a row of its own.
//!
//! A run that fails is tried again after 1, 5 and 30 minutes; after the
//! fourth failure a job that runs once is `failed`, and a repeating one
//! moves on to its next time, its failures kept in its runs.

use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::Serialize;
use serde_json::Value;

use crate::data::docs::new_id;

/// The app the platform's own jobs belong to.
pub const PLATFORM: &str = "_platform";

/// How long after each failure a job is tried again.
pub const BACKOFF_MS: [i64; 3] = [60_000, 5 * 60_000, 30 * 60_000];

/// The runs kept of each job, newest.
const RUNS_KEPT: i64 = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Waiting,
    Running,
    Held,
    Done,
    Failed,
}

impl State {
    pub fn name(self) -> &'static str {
        match self {
            State::Waiting => "waiting",
            State::Running => "running",
            State::Held => "held",
            State::Done => "done",
            State::Failed => "failed",
        }
    }

    pub fn parse(s: &str) -> Option<State> {
        [
            State::Waiting,
            State::Running,
            State::Held,
            State::Done,
            State::Failed,
        ]
        .into_iter()
        .find(|st| st.name() == s)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Job {
    pub id: String,
    pub app: String,
    pub workflow: String,
    pub args: Value,
    pub source: String,
    pub rule: Option<String>,
    pub catch_up: bool,
    pub next_ms: Option<i64>,
    pub attempts: i64,
    pub state: State,
    pub run_as: Option<String>,
    pub updated_ms: i64,
}

/// A job to make.
#[derive(Debug, Clone)]
pub struct NewJob<'a> {
    pub app: &'a str,
    pub workflow: &'a str,
    pub args: &'a Value,
    pub source: &'a str,
    pub rule: Option<&'a str>,
    pub catch_up: bool,
    pub next_ms: i64,
    pub run_as: Option<&'a str>,
}

const COLUMNS: &str =
    "id, app, workflow, args, source, rule, catch_up, next_ms, attempts, state, run_as, updated_ms";

fn job(r: &Row<'_>) -> rusqlite::Result<Job> {
    let args: String = r.get(3)?;
    let state: String = r.get(9)?;
    Ok(Job {
        id: r.get(0)?,
        app: r.get(1)?,
        workflow: r.get(2)?,
        args: serde_json::from_str(&args).unwrap_or(Value::Null),
        source: r.get(4)?,
        rule: r.get(5)?,
        catch_up: r.get(6)?,
        next_ms: r.get(7)?,
        attempts: r.get(8)?,
        state: State::parse(&state).unwrap_or(State::Failed),
        run_as: r.get(10)?,
        updated_ms: r.get(11)?,
    })
}

/// Make `new`, waiting; answers its id.
pub fn insert(conn: &Connection, new: &NewJob<'_>, now: i64) -> Result<String> {
    let id = new_id(now);
    conn.execute(
        "INSERT INTO jobs (id, app, workflow, args, source, rule, catch_up, next_ms, attempts, state, run_as, updated_ms) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, 'waiting', ?9, ?10)",
        params![
            id,
            new.app,
            new.workflow,
            new.args.to_string(),
            new.source,
            new.rule,
            new.catch_up,
            new.next_ms,
            new.run_as,
            now
        ],
    )?;
    Ok(id)
}

pub fn get(conn: &Connection, id: &str) -> Result<Option<Job>> {
    let sql = format!("SELECT {COLUMNS} FROM jobs WHERE id = ?1");
    Ok(conn.query_row(&sql, params![id], job).optional()?)
}

/// App `app`'s jobs from `source`, which a source prefix like `record:0:`
/// selects when it ends in `:`.
pub fn of_source(conn: &Connection, app: &str, source: &str) -> Result<Vec<Job>> {
    let sql = format!(
        "SELECT {COLUMNS} FROM jobs WHERE app = ?1 AND (source = ?2 OR (substr(?2, -1) = ':' AND \
         substr(source, 1, length(?2)) = ?2)) ORDER BY source, id"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![app, source], job)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// The jobs, by app and state when asked, soonest first.
pub fn list(conn: &Connection, app: Option<&str>, state: Option<State>) -> Result<Vec<Job>> {
    let sql = format!(
        "SELECT {COLUMNS} FROM jobs WHERE (?1 IS NULL OR app = ?1) AND (?2 IS NULL OR state = ?2) \
         ORDER BY next_ms IS NULL, next_ms, id"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![app, state.map(State::name)], job)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// When the soonest waiting job of an app in `apps` is due.
pub fn earliest(conn: &Connection, apps: &[String]) -> Result<Option<i64>> {
    let apps = serde_json::to_string(apps).expect("strings serialize");
    Ok(conn.query_row(
        "SELECT MIN(next_ms) FROM jobs WHERE state = 'waiting' AND next_ms IS NOT NULL \
         AND app IN (SELECT value FROM json_each(?1))",
        params![apps],
        |r| r.get(0),
    )?)
}

/// Take up to `n` waiting jobs of the apps in `apps` due by `now`, soonest
/// first, marking them running.
pub fn claim(conn: &mut Connection, apps: &[String], now: i64, n: usize) -> Result<Vec<Job>> {
    let apps = serde_json::to_string(apps).expect("strings serialize");
    let tx = conn.transaction()?;
    let sql = format!(
        "SELECT {COLUMNS} FROM jobs WHERE state = 'waiting' AND next_ms <= ?1 \
         AND app IN (SELECT value FROM json_each(?2)) ORDER BY next_ms, id LIMIT ?3"
    );
    let due: Vec<Job> = tx
        .prepare(&sql)?
        .query_map(params![now, apps, n as i64], job)?
        .collect::<Result<_, _>>()?;
    for j in &due {
        tx.execute(
            "UPDATE jobs SET state = 'running', updated_ms = ?2 WHERE id = ?1",
            params![j.id, now],
        )?;
    }
    tx.commit()?;
    Ok(due
        .into_iter()
        .map(|j| Job {
            state: State::Running,
            updated_ms: now,
            ..j
        })
        .collect())
}

/// A run of `job` ended well: a repeating job waits for `next`, one that
/// runs once is done. A job its record moved while it ran is left at the
/// new time ([`crate::jobs::follow`]), as it is no longer running.
pub fn succeeded(conn: &Connection, job: &Job, next: Option<i64>, now: i64) -> Result<()> {
    let (state, at) = match next {
        Some(at) => (State::Waiting, Some(at)),
        None => (State::Done, None),
    };
    conn.execute(
        "UPDATE jobs SET next_ms = ?2, attempts = 0, state = ?3, updated_ms = ?4 \
         WHERE id = ?1 AND state = 'running'",
        params![job.id, at, state.name(), now],
    )?;
    Ok(())
}

/// A run of `job` failed: it is tried again after its backoff, or — its
/// tries spent — a repeating one moves to `next` and one that runs once is
/// failed. Answers the state it is left in.
pub fn failed(conn: &Connection, job: &Job, next: Option<i64>, now: i64) -> Result<State> {
    let attempts = job.attempts + 1;
    let (state, at, attempts) = match (BACKOFF_MS.get(job.attempts as usize), next) {
        (Some(wait), _) => (State::Waiting, Some(now + wait), attempts),
        (None, Some(at)) => (State::Waiting, Some(at), 0),
        (None, None) => (State::Failed, None, attempts),
    };
    conn.execute(
        "UPDATE jobs SET next_ms = ?2, attempts = ?3, state = ?4, updated_ms = ?5 \
         WHERE id = ?1 AND state = 'running'",
        params![job.id, at, attempts, state.name(), now],
    )?;
    Ok(state)
}

/// What an admin asks of a job (§6.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Act {
    /// Run it now, whatever its state but running.
    Run,
    Hold,
    Release,
}

/// The job after an admin's act, or why the act was refused.
pub type Acted = std::result::Result<Job, &'static str>;

/// Do `act` to job `id`; answers the job after, or why not — `None` when
/// there is no such job.
pub fn act(conn: &Connection, id: &str, act: Act, now: i64) -> Result<Option<Acted>> {
    let Some(j) = get(conn, id)? else {
        return Ok(None);
    };
    let changed = match (act, j.state) {
        (_, State::Running) => return Ok(Some(Err("it is running now"))),
        (Act::Run, _) => conn.execute(
            "UPDATE jobs SET state = 'waiting', next_ms = ?2, attempts = 0, updated_ms = ?2 WHERE id = ?1",
            params![id, now],
        )?,
        (Act::Hold, State::Waiting) => conn.execute(
            "UPDATE jobs SET state = 'held', updated_ms = ?2 WHERE id = ?1",
            params![id, now],
        )?,
        (Act::Hold, _) => return Ok(Some(Err("only a waiting job is held"))),
        (Act::Release, State::Held) => conn.execute(
            "UPDATE jobs SET state = CASE WHEN next_ms IS NULL THEN 'done' ELSE 'waiting' END, \
             updated_ms = ?2 WHERE id = ?1",
            params![id, now],
        )?,
        (Act::Release, _) => return Ok(Some(Err("only a held job is released"))),
    };
    debug_assert_eq!(changed, 1);
    Ok(get(conn, id)?.map(Ok))
}

/// A job left running by a site that stopped is due again.
pub fn requeue_running(conn: &Connection, now: i64) -> Result<usize> {
    Ok(conn.execute(
        "UPDATE jobs SET state = 'waiting', updated_ms = ?1 WHERE state = 'running'",
        params![now],
    )?)
}

/// Drop jobs that ran once and are done, and their runs, older than
/// `before`.
pub fn tidy(conn: &Connection, before: i64) -> Result<usize> {
    let gone = "SELECT id FROM jobs WHERE state = 'done' AND rule IS NULL AND updated_ms < ?1";
    conn.execute(
        &format!("DELETE FROM job_runs WHERE job IN ({gone})"),
        params![before],
    )?;
    Ok(conn.execute(
        "DELETE FROM jobs WHERE state = 'done' AND rule IS NULL AND updated_ms < ?1",
        params![before],
    )?)
}

/// How a run went.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Run {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job: Option<String>,
    pub app: String,
    pub workflow: String,
    pub started_ms: i64,
    pub ended_ms: i64,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub log: Vec<String>,
}

/// Keep `run`, and only the newest of its job's runs.
pub fn record(conn: &Connection, run: &Run) -> Result<()> {
    conn.execute(
        "INSERT INTO job_runs (job, app, workflow, started_ms, ended_ms, outcome, error, log) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            run.job,
            run.app,
            run.workflow,
            run.started_ms,
            run.ended_ms,
            if run.ok { "ok" } else { "failed" },
            run.error,
            serde_json::to_string(&run.log).expect("strings serialize"),
        ],
    )?;
    if let Some(job) = &run.job {
        conn.execute(
            "DELETE FROM job_runs WHERE job = ?1 AND id NOT IN \
             (SELECT id FROM job_runs WHERE job = ?1 ORDER BY started_ms DESC, id DESC LIMIT ?2)",
            params![job, RUNS_KEPT],
        )?;
    }
    Ok(())
}

/// Job `job`'s runs, newest first, at most `n`.
pub fn runs(conn: &Connection, job: &str, n: usize) -> Result<Vec<Run>> {
    let mut stmt = conn.prepare(
        "SELECT job, app, workflow, started_ms, ended_ms, outcome, error, log FROM job_runs \
         WHERE job = ?1 ORDER BY started_ms DESC, id DESC LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![job, n as i64], |r| {
        let outcome: String = r.get(5)?;
        let log: String = r.get(7)?;
        Ok(Run {
            job: r.get(0)?,
            app: r.get(1)?,
            workflow: r.get(2)?,
            started_ms: r.get(3)?,
            ended_ms: r.get(4)?,
            ok: outcome == "ok",
            error: r.get(6)?,
            log: serde_json::from_str(&log).unwrap_or_default(),
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::store::Store;

    fn new<'a>(source: &'a str, rule: Option<&'a str>, args: &'a Value) -> NewJob<'a> {
        NewJob {
            app: "home",
            workflow: "remind",
            args,
            source,
            rule,
            catch_up: false,
            next_ms: 100,
            run_as: Some("1"),
        }
    }

    fn apps() -> Vec<String> {
        vec!["home".into()]
    }

    /// **A due job is claimed once, runs, and is done; a repeating one waits
    /// for its next time.**
    #[test]
    fn a_due_job_is_claimed_once_and_finished() {
        let store = Store::in_memory().unwrap();
        let mut conn = store.conn();
        let args = json!({"n": 1});
        let once = insert(&conn, &new("workflow", None, &args), 1).unwrap();
        let daily = insert(&conn, &new("schedule:0", Some("daily 07:30"), &args), 1).unwrap();
        assert_eq!(earliest(&conn, &apps()).unwrap(), Some(100));
        assert_eq!(
            earliest(&conn, &["other".into()]).unwrap(),
            None,
            "not served"
        );
        assert!(
            claim(&mut conn, &apps(), 99, 4).unwrap().is_empty(),
            "not due"
        );
        let claimed = claim(&mut conn, &apps(), 100, 4).unwrap();
        assert_eq!(claimed.len(), 2);
        assert_eq!(claimed[0].args, args);
        assert!(
            claim(&mut conn, &apps(), 100, 4).unwrap().is_empty(),
            "claimed once"
        );
        succeeded(
            &conn,
            &claimed.iter().find(|j| j.id == once).unwrap().clone(),
            None,
            200,
        )
        .unwrap();
        succeeded(
            &conn,
            &claimed.iter().find(|j| j.id == daily).unwrap().clone(),
            Some(900),
            200,
        )
        .unwrap();
        let o = get(&conn, &once).unwrap().unwrap();
        assert_eq!((o.state, o.next_ms), (State::Done, None));
        let d = get(&conn, &daily).unwrap().unwrap();
        assert_eq!((d.state, d.next_ms), (State::Waiting, Some(900)));
    }

    /// **A failure is tried again after 1, 5 and 30 minutes, then the job
    /// fails — or, repeating, moves to its next time.**
    #[test]
    fn a_failure_backs_off_then_fails() {
        let store = Store::in_memory().unwrap();
        let conn = store.conn();
        let args = json!({});
        let id = insert(&conn, &new("workflow", None, &args), 1).unwrap();
        let running = |id: &str| {
            conn.execute("UPDATE jobs SET state = 'running' WHERE id = ?1", [id])
                .unwrap();
        };
        let mut seen = Vec::new();
        for _ in 0..4 {
            running(&id);
            let j = get(&conn, &id).unwrap().unwrap();
            let st = failed(&conn, &j, None, 1_000).unwrap();
            let j = get(&conn, &id).unwrap().unwrap();
            seen.push((st, j.next_ms, j.attempts));
        }
        assert_eq!(
            seen,
            [
                (State::Waiting, Some(61_000), 1),
                (State::Waiting, Some(301_000), 2),
                (State::Waiting, Some(1_801_000), 3),
                (State::Failed, None, 4),
            ]
        );
        let rid = insert(&conn, &new("schedule:0", Some("hourly"), &args), 1).unwrap();
        conn.execute("UPDATE jobs SET attempts = 3 WHERE id = ?1", [&rid])
            .unwrap();
        running(&rid);
        let j = get(&conn, &rid).unwrap().unwrap();
        assert_eq!(
            failed(&conn, &j, Some(5_000), 1_000).unwrap(),
            State::Waiting
        );
        let j = get(&conn, &rid).unwrap().unwrap();
        assert_eq!((j.next_ms, j.attempts), (Some(5_000), 0));
    }

    /// **An admin holds, releases and runs a job now**; a running one is
    /// left alone.
    #[test]
    fn an_admin_holds_releases_and_runs_a_job() {
        let store = Store::in_memory().unwrap();
        let mut conn = store.conn();
        let args = json!({});
        let id = insert(&conn, &new("workflow", None, &args), 1).unwrap();
        let held = act(&conn, &id, Act::Hold, 5).unwrap().unwrap().unwrap();
        assert_eq!(held.state, State::Held);
        assert!(
            claim(&mut conn, &apps(), 1_000, 4).unwrap().is_empty(),
            "held"
        );
        assert_eq!(
            act(&conn, &id, Act::Hold, 5).unwrap().unwrap(),
            Err("only a waiting job is held")
        );
        assert_eq!(
            act(&conn, &id, Act::Release, 6)
                .unwrap()
                .unwrap()
                .unwrap()
                .state,
            State::Waiting
        );
        let now = act(&conn, &id, Act::Run, 50).unwrap().unwrap().unwrap();
        assert_eq!((now.state, now.next_ms), (State::Waiting, Some(50)));
        claim(&mut conn, &apps(), 50, 4).unwrap();
        assert_eq!(
            act(&conn, &id, Act::Run, 60).unwrap().unwrap(),
            Err("it is running now")
        );
        assert_eq!(act(&conn, "nope", Act::Run, 60).unwrap(), None);
        assert_eq!(requeue_running(&conn, 70).unwrap(), 1);
        assert_eq!(get(&conn, &id).unwrap().unwrap().state, State::Waiting);
    }

    /// **A job's runs are kept, newest first, the newest twenty.**
    #[test]
    fn a_jobs_runs_are_kept() {
        let store = Store::in_memory().unwrap();
        let conn = store.conn();
        for i in 0..25 {
            let run = Run {
                job: Some("j".into()),
                app: "home".into(),
                workflow: "remind".into(),
                started_ms: i,
                ended_ms: i + 3,
                ok: i % 2 == 0,
                error: (i % 2 == 1).then(|| "boom".to_string()),
                log: vec![format!("line {i}")],
            };
            record(&conn, &run).unwrap();
        }
        let kept = runs(&conn, "j", 100).unwrap();
        assert_eq!(kept.len(), 20);
        assert_eq!(kept[0].started_ms, 24);
        assert_eq!(kept[0].log, ["line 24"]);
        assert_eq!(kept[1].error.as_deref(), Some("boom"));
    }

    /// **A job that ran once and is long done is tidied away.**
    #[test]
    fn done_jobs_are_tidied() {
        let store = Store::in_memory().unwrap();
        let conn = store.conn();
        let args = json!({});
        let old = insert(&conn, &new("workflow", None, &args), 1).unwrap();
        let daily = insert(&conn, &new("schedule:0", Some("hourly"), &args), 1).unwrap();
        conn.execute("UPDATE jobs SET state = 'running' WHERE id = ?1", [&old])
            .unwrap();
        let j = get(&conn, &old).unwrap().unwrap();
        succeeded(&conn, &j, None, 10).unwrap();
        let other = insert(&conn, &new("workflow", None, &args), 1).unwrap();
        let waiting = get(&conn, &other).unwrap().unwrap();
        succeeded(&conn, &waiting, None, 10).unwrap();
        let left = get(&conn, &other).unwrap().unwrap();
        assert_eq!(
            left.state,
            State::Waiting,
            "a job no longer running is left as it is"
        );
        conn.execute("UPDATE jobs SET state = 'done' WHERE id = ?1", [&daily])
            .unwrap();
        assert_eq!(tidy(&conn, 11).unwrap(), 1);
        assert!(get(&conn, &old).unwrap().is_none());
        assert!(
            get(&conn, &daily).unwrap().is_some(),
            "a repeating job is kept"
        );
        assert_eq!(of_source(&conn, "home", "schedule:").unwrap().len(), 1);
        assert_eq!(of_source(&conn, "home", "schedule:0").unwrap().len(), 1);
        assert_eq!(
            of_source(&conn, "home", "schedule").unwrap().len(),
            0,
            "exact"
        );
    }
}
