//! `GET /api/v1/events/_reload` (§5.3): a server-sent event each time the
//! site starts serving a new commit, so an open page reloads itself and Yen
//! sees an edit without touching her phone.

use std::convert::Infallible;
use std::sync::Arc;

use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures_util::stream::Stream;
use futures_util::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

use crate::app::App;

pub async fn reload(
    State(app): State<Arc<App>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let moves = BroadcastStream::new(app.reloads.subscribe()).filter_map(|m| async move {
        // A page that fell behind missed a move it reloads for anyway.
        let commit = m.unwrap_or_default();
        Some(Ok(Event::default().event("reload").data(commit)))
    });
    Sse::new(moves).keep_alive(KeepAlive::default())
}
