//! One job run (§9.3): the workflow it names, as the account it acts for
//! or as the app, with its arguments — a record's job is handed the record
//! as it is now — and then its outcome kept: the run in `job_runs`, a line
//! in the app's log, and the job done, due again, or tried again later.

use std::sync::Arc;

use anyhow::{anyhow, Error, Result};
use serde_json::{json, Value};

use super::clock::spoken;
use super::follow::due_of;
use super::platform;
use super::rule::Rule;
use super::table::{self, Job, Run, State, PLATFORM};
use crate::accounts::caller::{acting_for, Caller};
use crate::accounts::now_ms;
use crate::app::App;
use crate::apps::manifest::Manifest;
use crate::apps::schedule::Kind;
use crate::data::docs::{self, Doc};
use crate::logs::browser::source;
use crate::logs::{Level, Line, PLATFORM as PLATFORM_LOG};
use crate::workflows::engine::Failure;
use crate::workflows::run::{run, Ask, Env, JobRef, Outcome};
use crate::workflows::sources;

/// What every run of the app's workflows reaches.
pub fn env_of(app: &App) -> Env {
    Env {
        store: app.store.clone(),
        changes: app.changes.clone(),
        zone: app.cfg.zone(),
    }
}

/// The caller job `job` acts as, or why it cannot be run.
fn caller_of(app: &App, m: &Manifest, run_as: Option<&str>) -> Result<Caller> {
    match run_as {
        None => Ok(Caller::app(&m.id, &m.name)),
        Some(id) => acting_for(&app.store, &app.cfg, id)?
            .ok_or_else(|| anyhow!("it acts for account {id}, which is no longer active")),
    }
}

/// The schedule number and record a record's job is of: `record:<n>:<coll>/<id>`.
fn record_of(job: &Job) -> Option<(usize, String, String)> {
    let rest = job.source.strip_prefix("record:")?;
    let (n, place) = rest.split_once(':')?;
    let (coll, id) = place.split_once('/')?;
    Some((n.parse().ok()?, coll.to_string(), id.to_string()))
}

/// When record `doc`'s job is next due after this run, if its time is
/// still to come — a job run early by an admin waits for it again.
fn record_next(m: &Manifest, n: usize, doc: &Doc, now: i64) -> Option<i64> {
    match m.schedules.get(n)?.kind(&m.collections).ok()? {
        Kind::From { at, before_ms, .. } => due_of(doc, at, before_ms, now),
        _ => None,
    }
}

/// What a run of `job` did, and when it is due next if it succeeded and
/// if it failed.
struct Ran {
    outcome: Outcome,
    next_ok: Option<i64>,
    next_failed: Option<i64>,
}

fn ran_with(answer: Result<Value, Failure>, log: Vec<String>, started_ms: i64) -> Outcome {
    Outcome {
        answer,
        log,
        scheduled: false,
        started_ms,
        ended_ms: now_ms(),
    }
}

/// Run app job `job`; `Ok(None)` when its record has gone and there is
/// nothing to run.
fn run_app_job(app: &App, job: &Job, now: i64) -> Result<Option<Ran>> {
    let m = app
        .apps
        .get(&job.app)
        .ok_or_else(|| anyhow!("the app `{}` is not served", job.app))?;
    let next_repeat = repeat_next(app, job, now);
    let caller = caller_of(app, &m, job.run_as.as_deref())?;
    let (args, next_ok) = match record_of(job) {
        Some((n, coll, id)) => {
            let found = docs::get(
                &app.store.conn(),
                &m,
                &coll,
                &Caller::app(&m.id, &m.name),
                &id,
            )?;
            let Ok(doc) = found else {
                return Ok(None);
            };
            (
                json!({"record": doc.record()}),
                record_next(&m, n, &doc, now),
            )
        }
        None => (job.args.clone(), next_repeat),
    };
    let sources = sources::of(&app.apps, &job.app)?;
    let ask = Ask {
        workflow: job.workflow.clone(),
        args,
        caller,
        job: job.next_ms.map(|due_ms| JobRef {
            id: job.id.clone(),
            due_ms,
        }),
    };
    let outcome = run(&env_of(app), m, sources, ask);
    Ok(Some(Ran {
        outcome,
        next_ok,
        next_failed: next_repeat,
    }))
}

/// Run `job`, claimed by the scheduler, and keep how it went.
pub fn run_job(app: &App, job: &Job) {
    let now = now_ms();
    let ran = if job.app == PLATFORM {
        let log = platform::run(&job.workflow, &app.store, &app.cfg, now);
        let next = repeat_next(app, job, now);
        let outcome = match log {
            Ok(log) => ran_with(Ok(Value::Null), log, now),
            Err(e) => ran_with(Err(failure(&e)), Vec::new(), now),
        };
        Ok(Some(Ran {
            outcome,
            next_ok: next,
            next_failed: next,
        }))
    } else {
        run_app_job(app, job, now)
    };
    let ran = match ran {
        Ok(Some(ran)) => ran,
        Ok(None) => {
            let done = table::succeeded(&app.store.conn(), job, None, now_ms());
            if let Err(e) = done {
                tracing::error!(target: "yenbot::jobs", job = %job.id, "could not be marked done: {e:#}");
            }
            return;
        }
        Err(e) => Ran {
            outcome: ran_with(Err(failure(&e)), Vec::new(), now),
            next_ok: None,
            next_failed: repeat_next(app, job, now),
        },
    };
    if let Err(e) = finish(app, job, &ran) {
        tracing::error!(target: "yenbot::jobs", job = %job.id, "its outcome could not be kept: {e:#}");
    }
    if ran.outcome.scheduled {
        app.jobs.wake();
    }
}

/// A repeating job's next time after `now`.
fn repeat_next(app: &App, job: &Job, now: i64) -> Option<i64> {
    Rule::parse(job.rule.as_deref()?)
        .ok()?
        .next(now, &app.cfg.zone())
}

fn failure(e: &Error) -> Failure {
    Failure {
        at: None,
        message: format!("{e:#}"),
    }
}

/// Keep how `job`'s run went: its run, its state, a line in its app's log.
fn finish(app: &App, job: &Job, ran: &Ran) -> Result<()> {
    let conn = app.store.conn();
    let left = match &ran.outcome.answer {
        Ok(_) => {
            table::succeeded(&conn, job, ran.next_ok, ran.outcome.ended_ms)?;
            None
        }
        Err(_) => Some(table::failed(
            &conn,
            job,
            ran.next_failed,
            ran.outcome.ended_ms,
        )?),
    };
    drop(conn);
    let after = match left {
        None => String::new(),
        Some(State::Failed) => "; it will not be tried again".to_string(),
        Some(_) => match table::get(&app.store.conn(), &job.id)?.and_then(|j| j.next_ms) {
            Some(at) => format!(
                "; next tried {}",
                spoken(at, &app.cfg.zone()).unwrap_or_default()
            ),
            None => String::new(),
        },
    };
    keep(
        app,
        Some(&job.id),
        &job.app,
        &job.workflow,
        &ran.outcome,
        &after,
    )
}

/// Keep a run of `workflow` of app `app` — a job's, or one asked for at
/// once — in `job_runs` and the app's log; `after` is said after its
/// outcome.
pub fn keep(
    app: &App,
    job: Option<&str>,
    app_id: &str,
    workflow: &str,
    outcome: &Outcome,
    after: &str,
) -> Result<()> {
    let run = Run {
        job: job.map(str::to_string),
        app: app_id.to_string(),
        workflow: workflow.to_string(),
        started_ms: outcome.started_ms,
        ended_ms: outcome.ended_ms,
        ok: outcome.answer.is_ok(),
        error: outcome.answer.as_ref().err().map(Failure::to_string),
        log: outcome.log.clone(),
    };
    table::record(&app.store.conn(), &run)?;
    let took = outcome.ended_ms - outcome.started_ms;
    let of = job.map(|j| format!(" (job {j})")).unwrap_or_default();
    let (level, said) = match &outcome.answer {
        Ok(_) => (
            Level::Info,
            format!("workflow {workflow}{of} ran in {took} ms{after}"),
        ),
        Err(f) => (
            Level::Error,
            format!("workflow {workflow}{of} failed in {took} ms: {f}{after}"),
        ),
    };
    let mut message = said;
    for line in &outcome.log {
        message.push('\n');
        message.push_str(line);
    }
    let log_source = if app_id == PLATFORM {
        PLATFORM_LOG.to_string()
    } else {
        source(app_id)
    };
    app.logs.write(&[Line {
        at_ms: outcome.ended_ms,
        level,
        source: log_source,
        target: "workflow".into(),
        message,
    }])?;
    Ok(())
}

/// Run workflow `workflow` of app `m` at once, as `caller`, with `args`,
/// and keep how it went (§10.4).
pub fn run_now(
    app: &App,
    m: Arc<Manifest>,
    workflow: &str,
    args: Value,
    caller: Caller,
) -> Result<Outcome> {
    let sources = sources::of(&app.apps, &m.id)?;
    let id = m.id.clone();
    let ask = Ask {
        workflow: workflow.to_string(),
        args,
        caller,
        job: None,
    };
    let outcome = run(&env_of(app), m, sources, ask);
    keep(app, None, &id, workflow, &outcome, "")?;
    if outcome.scheduled {
        app.jobs.wake();
    }
    Ok(outcome)
}
