//! `/api/v1/ai/…` (§6.1, §14): a chat with the model, a picture drawn or
//! changed, and a question about a collection — for the app the request
//! names (`X-Yen-App`, or `?app=` for a token), as the caller, only for an
//! app with `ai: true`. Asked `stream: true`, a chat's answer and a
//! picture's progress arrive as server-sent events as they happen.

use std::convert::Infallible;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use super::data::opened;
use super::reply::{active, body, error, internal, ok, Args, APP_HEADER};
use crate::accounts::caller::Caller;
use crate::ai::chat::Ask;
use crate::ai::image::Draw;
use crate::ai::Asker;
use crate::app::App;
use crate::apps::manifest::Manifest;

#[derive(Debug, Default, Deserialize)]
pub struct ForApp {
    #[serde(default)]
    app: Option<String>,
}

/// The app a request is for: its `X-Yen-App`, or `?app=`.
fn app_of(headers: &HeaderMap, q: &ForApp) -> Result<String, Response> {
    headers
        .get(APP_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
        .or_else(|| q.app.clone())
        .ok_or_else(|| {
            error(
                StatusCode::BAD_REQUEST,
                "no_app",
                "say which app asks: the `X-Yen-App` header, or `?app=`",
            )
        })
}

/// The caller and the app, when the caller may open it.
async fn for_app(
    s: &Arc<App>,
    method: &Method,
    headers: &HeaderMap,
    q: &ForApp,
) -> Result<(Caller, Arc<Manifest>), Response> {
    let app = app_of(headers, q)?;
    let caller = active(s, method, headers).await?;
    let (s, c) = (s.clone(), caller.clone());
    let m = tokio::task::spawn_blocking(move || opened(&s, &app, &c))
        .await
        .map_err(internal)??;
    Ok((caller, m))
}

/// A refusal of the app's, said for it.
fn refused(why: String) -> Response {
    error(StatusCode::UNPROCESSABLE_ENTITY, "refused", why)
}

/// Run `work` off the request, its events streamed as they come, ending
/// with what it answers — `done`, or `error`.
fn streamed<F>(work: F) -> Response
where
    F: FnOnce(&mut dyn FnMut(&str, Value)) -> (&'static str, Value) + Send + 'static,
{
    let (tx, rx) = mpsc::channel::<Result<Event, Infallible>>(64);
    tokio::task::spawn_blocking(move || {
        let mut send = |name: &str, v: Value| {
            let _ = tx.blocking_send(Ok(Event::default().event(name).data(v.to_string())));
        };
        let (name, last) = work(&mut send);
        send(name, last);
    });
    Sse::new(ReceiverStream::new(rx))
        .keep_alive(KeepAlive::default())
        .into_response()
}

#[derive(Deserialize)]
struct Chat {
    #[serde(flatten)]
    ask: Ask,
    #[serde(default)]
    stream: bool,
}

/// `POST /api/v1/ai/chat`.
pub async fn chat(
    State(s): State<Arc<App>>,
    Args(q): Args<ForApp>,
    method: Method,
    headers: HeaderMap,
    raw: Bytes,
) -> Response {
    let asked: Chat = match body(&raw).and_then(|v| {
        serde_json::from_value(v).map_err(|e| {
            error(
                StatusCode::BAD_REQUEST,
                "bad_request",
                format!("a chat is `{{messages, system?, max_tokens?, schema?, stream?}}`: {e}"),
            )
        })
    }) {
        Ok(a) => a,
        Err(r) => return r,
    };
    let (caller, m) = match for_app(&s, &method, &headers, &q).await {
        Ok(x) => x,
        Err(r) => return r,
    };
    let answer = move |s: &App, delta: &mut dyn FnMut(&str)| {
        let asker = Asker {
            zen: s.zen.as_ref(),
            store: &s.store,
            files: &s.files,
            m: &m,
            caller: &caller,
        };
        asker.chat(&asked.ask, delta)
    };
    if asked.stream {
        return streamed(move |send| {
            match answer(&s, &mut |piece| send("delta", json!({"content": piece}))) {
                Ok(Ok(a)) => ("done", json!({"content": a.content, "json": a.json})),
                Ok(Err(why)) => ("error", json!({"error": "refused", "detail": why})),
                Err(e) => (
                    "error",
                    json!({"error": "internal", "detail": format!("{e:#}")}),
                ),
            }
        });
    }
    tokio::task::spawn_blocking(move || match answer(&s, &mut |_| {}) {
        Ok(Ok(a)) => ok(json!({
            "content": a.content, "json": a.json,
            "usage": {"tokens_in": a.tokens_in, "tokens_out": a.tokens_out},
        })),
        Ok(Err(why)) => refused(why),
        Err(e) => internal(e),
    })
    .await
    .unwrap_or_else(internal)
}

#[derive(Deserialize)]
struct Picture {
    #[serde(flatten)]
    draw: Draw,
    #[serde(default)]
    stream: bool,
}

/// `POST /api/v1/ai/image`.
pub async fn image(
    State(s): State<Arc<App>>,
    Args(q): Args<ForApp>,
    method: Method,
    headers: HeaderMap,
    raw: Bytes,
) -> Response {
    let asked: Picture = match body(&raw).and_then(|v| {
        serde_json::from_value(v).map_err(|e| {
            error(
                StatusCode::BAD_REQUEST,
                "bad_request",
                format!(
                    "a picture is `{{prompt, width?, height?, seed?, from?, hold?, stream?}}`: {e}"
                ),
            )
        })
    }) {
        Ok(a) => a,
        Err(r) => return r,
    };
    let (caller, m) = match for_app(&s, &method, &headers, &q).await {
        Ok(x) => x,
        Err(r) => return r,
    };
    let drawn = move |s: &App, progress: &mut dyn FnMut(&Value)| {
        let asker = Asker {
            zen: s.zen.as_ref(),
            store: &s.store,
            files: &s.files,
            m: &m,
            caller: &caller,
        };
        asker.image(&asked.draw, progress)
    };
    if asked.stream {
        return streamed(
            move |send| match drawn(&s, &mut |v| send("progress", v.clone())) {
                Ok(Ok((file, seed))) => ("done", json!({"file": file.shown(), "seed": seed})),
                Ok(Err(why)) => ("error", json!({"error": "refused", "detail": why})),
                Err(e) => (
                    "error",
                    json!({"error": "internal", "detail": format!("{e:#}")}),
                ),
            },
        );
    }
    tokio::task::spawn_blocking(move || match drawn(&s, &mut |_| {}) {
        Ok(Ok((file, seed))) => ok(json!({"file": file.shown(), "seed": seed})),
        Ok(Err(why)) => refused(why),
        Err(e) => internal(e),
    })
    .await
    .unwrap_or_else(internal)
}

#[derive(Deserialize)]
struct Question {
    question: String,
    from: String,
}

/// `POST /api/v1/ai/ask`.
pub async fn ask(
    State(s): State<Arc<App>>,
    Args(q): Args<ForApp>,
    method: Method,
    headers: HeaderMap,
    raw: Bytes,
) -> Response {
    let asked: Question = match body(&raw).and_then(|v| {
        serde_json::from_value(v).map_err(|e| {
            error(
                StatusCode::BAD_REQUEST,
                "bad_request",
                format!("a question is `{{question, from}}`: {e}"),
            )
        })
    }) {
        Ok(a) => a,
        Err(r) => return r,
    };
    let (caller, m) = match for_app(&s, &method, &headers, &q).await {
        Ok(x) => x,
        Err(r) => return r,
    };
    tokio::task::spawn_blocking(move || {
        let asker = Asker {
            zen: s.zen.as_ref(),
            store: &s.store,
            files: &s.files,
            m: &m,
            caller: &caller,
        };
        match asker.ask(&asked.from, &asked.question) {
            Ok(Ok(a)) => ok(a),
            Ok(Err(why)) => refused(why),
            Err(e) => internal(e),
        }
    })
    .await
    .unwrap_or_else(internal)
}
