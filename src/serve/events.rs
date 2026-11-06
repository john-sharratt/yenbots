//! `GET /api/v1/events/{app}` (§6.1): server-sent events, one `change` for
//! each write to the app's records the caller may see, and a `reset` when
//! the page should read everything again — after a restore, or when it fell
//! behind the feed.

use std::convert::Infallible;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, Method};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use futures_util::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

use super::data::opened;
use super::reply::{active, internal};
use crate::app::App;
use crate::data::changes::Feed;

/// The event a page is sent for `feed`.
pub fn event_of(feed: &Feed) -> Event {
    match feed {
        Feed::Changed(c) => Event::default()
            .event("change")
            .json_data(c)
            .unwrap_or_else(|_| Event::default().event("reset").data("{}")),
        Feed::Reset(_) => Event::default().event("reset").data("{}"),
    }
}

pub async fn changes(
    State(s): State<Arc<App>>,
    Path(app): Path<String>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    let caller = match active(&s, &method, &headers).await {
        Ok(c) => c,
        Err(r) => return r,
    };
    let (state, c) = (s.clone(), caller.clone());
    let m = match tokio::task::spawn_blocking(move || opened(&state, &app, &c)).await {
        Ok(Ok(m)) => m,
        Ok(Err(r)) => return r,
        Err(e) => return internal(e),
    };
    let heard = BroadcastStream::new(s.changes.subscribe()).filter_map(move |f| {
        let (m, caller) = (m.clone(), caller.clone());
        async move {
            match f {
                Ok(feed) if feed.reaches(&m, &caller) => Some(Ok::<_, Infallible>(event_of(&feed))),
                Ok(_) => None,
                // A page that fell behind missed changes it cannot name.
                Err(_) => Some(Ok(event_of(&Feed::Reset(None)))),
            }
        }
    });
    Sse::new(heard)
        .keep_alive(KeepAlive::default())
        .into_response()
}
