//! The scheduler (§9.3, §9.4): one loop that wakes at the soonest job due —
//! and at once on any write, any job made, any reload — claims what is due
//! and runs it on a bounded pool, a few at a time. Only the jobs of apps
//! being served are claimed, and the platform's own.
//!
//! On start it finds what fell due while it was down: a job due within the
//! last hour runs once; an older one runs once only if its schedule says
//! `catch_up: true`. A repeating job then moves to its next time rather
//! than running every one it missed; a missed one is kept as a run that
//! failed, so it shows on the admin page.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use rusqlite::{params, Connection};
use tokio::sync::{Notify, Semaphore};

use super::clock::spoken;
use super::platform;
use super::rule::Rule;
use super::runner::run_job;
use super::sync::sync;
use super::table::{self, Job, Run, State, PLATFORM};
use crate::accounts::now_ms;
use crate::app::App;

/// How many runs go at once.
pub const POOL: usize = 4;

/// How long ago a missed job may have fallen due and still run.
pub const RECENT_MS: i64 = 3_600_000;

/// The longest the loop sleeps without looking.
const LOOK: Duration = Duration::from_secs(60);

/// What wakes the scheduler.
#[derive(Clone, Default)]
pub struct Jobs {
    wake: Arc<Notify>,
}

impl Jobs {
    pub fn new() -> Jobs {
        Jobs::default()
    }

    /// Look again now: a job was made or changed.
    pub fn wake(&self) {
        self.wake.notify_one();
    }
}

/// Every served app's jobs brought into step with its manifest, and the
/// platform's made.
pub fn sync_all(app: &App, now: i64) -> Result<()> {
    let tz = app.cfg.zone();
    platform::sync(&app.store.conn(), &tz, now)?;
    for m in app.apps.all() {
        if let Err(e) = sync(&mut app.store.conn(), &m, &tz, now) {
            tracing::error!(target: "yenbot::jobs", app = %m.id, "its jobs could not be brought into step: {e:#}");
        }
    }
    Ok(())
}

/// Settle what fell due while the site was down, at `now`; answers the
/// jobs passed over.
pub fn catch_up(app: &App, now: i64) -> Result<Vec<String>> {
    let conn = app.store.conn();
    table::requeue_running(&conn, now)?;
    let missed: Vec<Job> = table::list(&conn, None, Some(State::Waiting))?
        .into_iter()
        .filter(|j| j.next_ms.is_some_and(|at| at < now - RECENT_MS) && !j.catch_up)
        .collect();
    let tz = app.cfg.zone();
    let mut passed = Vec::new();
    for j in missed {
        let due = spoken(j.next_ms.unwrap_or(now), &tz).unwrap_or_default();
        let next = j
            .rule
            .as_deref()
            .and_then(|r| Rule::parse(r).ok())
            .and_then(|r| r.next(now, &tz));
        pass_over(&conn, &j, &due, next, now)?;
        passed.push(j.id);
    }
    Ok(passed)
}

/// Keep `job`, missed at `due`, as a run that failed, and move it to `next`
/// — or, with none, leave it failed.
fn pass_over(conn: &Connection, job: &Job, due: &str, next: Option<i64>, now: i64) -> Result<()> {
    table::record(
        conn,
        &Run {
            job: Some(job.id.clone()),
            app: job.app.clone(),
            workflow: job.workflow.clone(),
            started_ms: now,
            ended_ms: now,
            ok: false,
            error: Some(format!(
                "missed: the site was down when it fell due, {due}, and came back over an hour later"
            )),
            log: Vec::new(),
        },
    )?;
    let (state, at) = match next {
        Some(at) => (State::Waiting, Some(at)),
        None => (State::Failed, None),
    };
    conn.execute(
        "UPDATE jobs SET state = ?2, next_ms = ?3, attempts = 0, updated_ms = ?4 WHERE id = ?1",
        params![job.id, state.name(), at, now],
    )?;
    Ok(())
}

/// The apps whose jobs are claimed: those served, and the platform.
fn claimable(app: &App) -> Vec<String> {
    let mut ids: Vec<String> = app.apps.all().iter().map(|m| m.id.clone()).collect();
    ids.push(PLATFORM.to_string());
    ids
}

/// Start the scheduler: settle what was missed, then run jobs as they fall
/// due for as long as the site runs.
pub fn start(app: Arc<App>) {
    tokio::spawn(async move {
        let first = app.clone();
        let settled = tokio::task::spawn_blocking(move || {
            let now = now_ms();
            sync_all(&first, now)?;
            catch_up(&first, now)
        })
        .await;
        match settled {
            Ok(Ok(passed)) if passed.is_empty() => {}
            Ok(Ok(passed)) => {
                tracing::warn!(target: "yenbot::jobs", jobs = ?passed, "passed over, missed while the site was down")
            }
            Ok(Err(e)) => {
                tracing::error!(target: "yenbot::jobs", "could not settle missed jobs: {e:#}")
            }
            Err(e) => tracing::error!(target: "yenbot::jobs", "settling missed jobs panicked: {e}"),
        }
        run_loop(app).await;
    });
}

async fn run_loop(app: Arc<App>) {
    let pool = Arc::new(Semaphore::new(POOL));
    let mut writes = app.changes.subscribe();
    loop {
        let free = pool.available_permits();
        let claimer = app.clone();
        let claimed = tokio::task::spawn_blocking(move || -> Result<(Vec<Job>, Option<i64>)> {
            let apps = claimable(&claimer);
            let now = now_ms();
            let due = if free > 0 {
                table::claim(&mut claimer.store.conn(), &apps, now, free)?
            } else {
                Vec::new()
            };
            let next = table::earliest(&claimer.store.conn(), &apps)?;
            Ok((due, next))
        })
        .await;
        let (due, next) = match claimed {
            Ok(Ok(c)) => c,
            Ok(Err(e)) => {
                tracing::error!(target: "yenbot::jobs", "could not claim due jobs: {e:#}");
                (Vec::new(), None)
            }
            Err(e) => {
                tracing::error!(target: "yenbot::jobs", "claiming panicked: {e}");
                (Vec::new(), None)
            }
        };
        let claimed_any = !due.is_empty();
        for job in due {
            let permit = pool
                .clone()
                .acquire_owned()
                .await
                .expect("the pool is never closed");
            let runner = app.clone();
            tokio::task::spawn_blocking(move || {
                run_job(&runner, &job);
                drop(permit);
                runner.jobs.wake();
            });
        }
        if claimed_any {
            continue;
        }
        // With every runner busy, a run's end is what wakes the loop.
        let wait = match next {
            Some(at) if free > 0 => Duration::from_millis((at - now_ms()).max(0) as u64).min(LOOK),
            _ => LOOK,
        };
        tokio::select! {
            _ = tokio::time::sleep(wait) => {}
            _ = app.jobs.wake.notified() => {}
            _ = writes.recv() => {}
        }
    }
}

#[cfg(test)]
#[path = "scheduler_tests.rs"]
mod tests;
