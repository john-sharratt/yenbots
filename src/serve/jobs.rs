//! Jobs and workflows over the API (§6.1, §6.2, §9, §10.4): an app's jobs
//! for its pages, every job for the admins, a job run now, held or
//! released, and a workflow run at once — as the caller, or, for an admin,
//! as the app.

use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use super::data::{no_app, opened};
use super::reply::{active, admin, body, error, internal, ok, Args};
use crate::accounts::caller::Caller;
use crate::accounts::now_ms;
use crate::app::App;
use crate::apps::manifest::Manifest;
use crate::jobs::runner::run_now;
use crate::jobs::table::{self, Act, Job, State as JobState};
use crate::workflows::run::Outcome;
use crate::workflows::sources;

/// The runs shown with each job.
const RUNS_SHOWN: usize = 5;

/// A job as the API answers it, with its last runs; its arguments only
/// when asked, as they may hold a record.
fn shown(s: &App, j: &Job, with_args: bool) -> Result<Value, Response> {
    let runs = table::runs(&s.store.conn(), &j.id, RUNS_SHOWN).map_err(internal)?;
    let mut v = json!({
        "id": j.id,
        "app": j.app,
        "workflow": j.workflow,
        "source": j.source,
        "rule": j.rule,
        "next_ms": j.next_ms,
        "state": j.state,
        "attempts": j.attempts,
        "run_as": j.run_as,
        "runs": runs,
    });
    if with_args {
        v["args"] = j.args.clone();
    }
    Ok(v)
}

/// `GET /api/v1/jobs/{app}`: the app's jobs the caller's pages may show —
/// those acting for the caller, and the app's own.
pub async fn of_app(
    State(s): State<Arc<App>>,
    Path(app): Path<String>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    let caller = match active(&s, &method, &headers).await {
        Ok(c) => c,
        Err(r) => return r,
    };
    tokio::task::spawn_blocking(move || {
        let m = match opened(&s, &app, &caller) {
            Ok(m) => m,
            Err(r) => return r,
        };
        let jobs = match table::list(&s.store.conn(), Some(&m.id), None) {
            Ok(j) => j,
            Err(e) => return internal(e),
        };
        let mut out = Vec::new();
        for j in jobs
            .iter()
            .filter(|j| j.run_as.as_deref().is_none_or(|a| a == caller.id()))
        {
            match shown(&s, j, false) {
                Ok(v) => out.push(v),
                Err(r) => return r,
            }
        }
        ok(json!({"jobs": out}))
    })
    .await
    .unwrap_or_else(internal)
}

/// A run's answer: its value and log, or — 422 — why it failed.
fn answered(outcome: Outcome) -> Response {
    let ms = outcome.ended_ms - outcome.started_ms;
    match outcome.answer {
        Ok(value) => ok(json!({"value": value, "log": outcome.log, "ms": ms})),
        Err(f) => {
            let body = json!({
                "error": "workflow_failed",
                "detail": f.to_string(),
                "log": outcome.log,
                "ms": ms,
            });
            (StatusCode::UNPROCESSABLE_ENTITY, Json(body)).into_response()
        }
    }
}

/// The arguments a run is given: the whole body, `{}` when there is none.
fn args_of(raw: &[u8]) -> Result<Value, Response> {
    if raw.is_empty() {
        return Ok(json!({}));
    }
    let v = body(raw)?;
    if !v.is_object() {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "bad_request",
            "a workflow's arguments are an object",
        ));
    }
    Ok(v)
}

/// Run workflow `name` of `m` at once as `caller`, answering how it went.
fn run_at_once(s: &App, m: Arc<Manifest>, name: &str, args: Value, caller: Caller) -> Response {
    match sources::of(&s.apps, &m.id) {
        Ok(src) if src.has(name) => {}
        Ok(_) => {
            return error(
                StatusCode::NOT_FOUND,
                "no_workflow",
                format!("{} has no workflow `{name}`", m.id),
            )
        }
        Err(e) => return internal(e),
    }
    match run_now(s, m, name, args, caller) {
        Ok(outcome) => answered(outcome),
        Err(e) => internal(e),
    }
}

/// `POST /api/v1/workflows/{app}/{name}`: run a workflow now, as the caller.
pub async fn run(
    State(s): State<Arc<App>>,
    Path((app, name)): Path<(String, String)>,
    method: Method,
    headers: HeaderMap,
    raw: Bytes,
) -> Response {
    let args = match args_of(&raw) {
        Ok(a) => a,
        Err(r) => return r,
    };
    let caller = match active(&s, &method, &headers).await {
        Ok(c) => c,
        Err(r) => return r,
    };
    tokio::task::spawn_blocking(move || match opened(&s, &app, &caller) {
        Ok(m) => run_at_once(&s, m, &name, args, caller),
        Err(r) => r,
    })
    .await
    .unwrap_or_else(internal)
}

/// `POST /api/v1/admin/workflows/{app}/{name}`: run a workflow now, as the
/// app.
pub async fn run_as_app(
    State(s): State<Arc<App>>,
    Path((app, name)): Path<(String, String)>,
    method: Method,
    headers: HeaderMap,
    raw: Bytes,
) -> Response {
    let args = match args_of(&raw) {
        Ok(a) => a,
        Err(r) => return r,
    };
    if let Err(r) = admin(&s, &method, &headers).await {
        return r;
    }
    tokio::task::spawn_blocking(move || {
        let Some(m) = s.apps.get(&app) else {
            return no_app(&app);
        };
        let caller = Caller::app(&m.id, &m.name);
        run_at_once(&s, m, &name, args, caller)
    })
    .await
    .unwrap_or_else(internal)
}

#[derive(Debug, Default, Deserialize)]
pub struct JobsQuery {
    #[serde(default)]
    app: Option<String>,
    #[serde(default)]
    state: Option<String>,
}

fn bad_state(st: &str) -> Response {
    error(
        StatusCode::BAD_REQUEST,
        "bad_state",
        format!("`{st}` is not a job's state: `waiting`, `running`, `held`, `done` or `failed`"),
    )
}

/// `GET /api/v1/admin/jobs`: every job, by app and state when asked.
pub async fn list(
    State(s): State<Arc<App>>,
    Args(q): Args<JobsQuery>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    if let Err(r) = admin(&s, &method, &headers).await {
        return r;
    }
    let state = match q.state.as_deref() {
        None => None,
        Some(st) => match JobState::parse(st) {
            Some(state) => Some(state),
            None => return bad_state(st),
        },
    };
    tokio::task::spawn_blocking(move || {
        let jobs = match table::list(&s.store.conn(), q.app.as_deref(), state) {
            Ok(j) => j,
            Err(e) => return internal(e),
        };
        let mut out = Vec::new();
        for j in &jobs {
            match shown(&s, j, true) {
                Ok(v) => out.push(v),
                Err(r) => return r,
            }
        }
        ok(json!({"jobs": out}))
    })
    .await
    .unwrap_or_else(internal)
}

/// `POST /api/v1/admin/jobs/{id}/{action}`: run a job now, hold it, or
/// release it.
pub async fn act(
    State(s): State<Arc<App>>,
    Path((id, action)): Path<(String, String)>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    let act = match action.as_str() {
        "run" => Act::Run,
        "hold" => Act::Hold,
        "release" => Act::Release,
        other => {
            return error(
                StatusCode::NOT_FOUND,
                "no_action",
                format!("a job is `run`, `hold` or `release`, not `{other}`"),
            )
        }
    };
    if let Err(r) = admin(&s, &method, &headers).await {
        return r;
    }
    tokio::task::spawn_blocking(move || {
        let done = table::act(&s.store.conn(), &id, act, now_ms());
        match done {
            Ok(None) => error(
                StatusCode::NOT_FOUND,
                "no_job",
                format!("there is no job `{id}`"),
            ),
            Ok(Some(Err(why))) => error(StatusCode::CONFLICT, "refused", why),
            Ok(Some(Ok(job))) => {
                s.jobs.wake();
                match shown(&s, &job, true) {
                    Ok(v) => ok(json!({"job": v})),
                    Err(r) => r,
                }
            }
            Err(e) => internal(e),
        }
    })
    .await
    .unwrap_or_else(internal)
}
