//! A workflow run (§10): `export default async function name(ctx, args)`
//! called with the app's `ctx` and the run's arguments, as the run's caller
//! — an account, or the app itself — within the app's deadline.

use std::sync::Arc;
use std::time::Duration;

use jiff::tz::TimeZone;
use serde::Serialize;
use serde_json::{json, Value};

use super::engine::{evaluate, Driver, Failure};
use super::host::Host;
use super::loader::Sources;
use crate::accounts::caller::Caller;
use crate::accounts::now_ms;
use crate::apps::manifest::Manifest;
use crate::config::Config;
use crate::data::changes::Changes;
use crate::jobs::follow::acting;
use crate::notify::push::Pusher;
use crate::store::Store;

/// What every run reaches.
#[derive(Clone)]
pub struct Env {
    pub store: Arc<Store>,
    pub changes: Changes,
    pub zone: TimeZone,
    /// The site's config: who the admins are, for `ctx.notify`.
    pub cfg: Config,
    pub pusher: Pusher,
}

/// The job a run is of, as `ctx.job` shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct JobRef {
    pub id: String,
    pub due_ms: i64,
}

/// A run to make.
pub struct Ask {
    pub workflow: String,
    pub args: Value,
    pub caller: Caller,
    pub job: Option<JobRef>,
}

/// How a run went.
#[derive(Debug)]
pub struct Outcome {
    /// What the workflow returned, or why it did not.
    pub answer: Result<Value, Failure>,
    /// What it wrote with `ctx.log`.
    pub log: Vec<String>,
    /// Whether it made a job with `ctx.schedule`.
    pub scheduled: bool,
    pub started_ms: i64,
    pub ended_ms: i64,
}

/// The driver that calls workflow `path` and leaves its answer. `yen:ctx`
/// is imported first, so the host function is off the global object before
/// any of the workflow's own code runs.
fn driver_of(path: &str) -> String {
    format!(
        "import {{ ctx }} from \"yen:ctx\";\nimport workflow from {};\n\
         globalThis.__yen_done = (async () => {{\n  \
           const value = await workflow(ctx, JSON.parse(globalThis.__yen_args));\n  \
           return JSON.stringify(value === undefined ? null : value);\n\
         }})();\n",
        json!(path)
    )
}

/// What `ctx` says of a run.
pub fn run_info(m: &Manifest, sources: &Sources, caller: &Caller, job: Option<&JobRef>) -> Value {
    json!({
        "app": m.id,
        "dir": sources.dir,
        "job": job,
        "account": acting(caller.id()),
    })
}

/// The host a run of app `m` calls, as `caller`.
pub fn host_for(env: &Env, m: Arc<Manifest>, sources: Sources, caller: Caller) -> Host {
    Host {
        store: env.store.clone(),
        changes: env.changes.clone(),
        manifest: m,
        caller,
        sources,
        zone: env.zone.clone(),
        cfg: env.cfg.clone(),
        pusher: env.pusher.clone(),
        log: Vec::new(),
        scheduled: false,
    }
}

/// Run `ask` of app `m`, whose workflow files are `sources`.
pub fn run(env: &Env, m: Arc<Manifest>, sources: Sources, ask: Ask) -> Outcome {
    let started_ms = now_ms();
    let ended = |answer: Result<Value, Failure>, log, scheduled| Outcome {
        answer,
        log,
        scheduled,
        started_ms,
        ended_ms: now_ms(),
    };
    if !sources.has(&ask.workflow) {
        let missing = Failure {
            at: None,
            message: format!("there is no workflow `{}`", ask.workflow),
        };
        return ended(Err(missing), Vec::new(), false);
    }
    let deadline = Duration::from_millis(m.deadline_ms().unwrap_or(60_000) as u64);
    let path = sources.path_of(&ask.workflow);
    let driver = Driver {
        source: driver_of(&path),
        path: &path,
        run: run_info(&m, &sources, &ask.caller, ask.job.as_ref()),
        args: ask.args,
    };
    let host = host_for(env, m, sources.clone(), ask.caller);
    let (host, answer) = evaluate(host, &sources, driver, deadline);
    ended(answer, host.log, host.scheduled)
}

#[cfg(test)]
#[path = "run_tests.rs"]
mod tests;
