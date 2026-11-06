//! The platform's `tracing` events as log lines, source `yenbot`: each event
//! at INFO or above becomes a [`Line`], handed to one writer thread that
//! stores them in batches, so logging never waits on the disk.

use std::fmt::{Debug, Write as _};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::Duration;

use tracing::field::{Field, Visit};
use tracing::{Event, Level as TraceLevel, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

use super::{Level, Line, Logs, PLATFORM};
use crate::accounts::now_ms;

/// How long the writer gathers lines before it stores them.
const BATCH: Duration = Duration::from_millis(250);

/// The layer, and the thread that stores what it sends.
pub struct StoreLayer {
    tx: Sender<Line>,
}

impl StoreLayer {
    /// A layer writing to `logs`, with its writer thread started.
    pub fn new(logs: Logs) -> StoreLayer {
        let (tx, rx) = channel();
        thread::Builder::new()
            .name("yenbot-logs".into())
            .spawn(move || write_loop(&logs, &rx))
            .expect("the log writer thread starts");
        StoreLayer { tx }
    }
}

fn write_loop(logs: &Logs, rx: &Receiver<Line>) {
    loop {
        let first = match rx.recv() {
            Ok(line) => line,
            Err(_) => return,
        };
        let mut batch = vec![first];
        loop {
            match rx.recv_timeout(BATCH) {
                Ok(line) => batch.push(line),
                Err(RecvTimeoutError::Timeout) => break,
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
        if let Err(e) = logs.write(&batch) {
            // The store is where lines go; with it failing, stderr is what is
            // left, and keep's log holds it.
            eprintln!("yenbot: {} log lines could not be stored: {e}", batch.len());
        }
    }
}

fn level_of(level: &TraceLevel) -> Level {
    match *level {
        TraceLevel::TRACE => Level::Trace,
        TraceLevel::DEBUG => Level::Debug,
        TraceLevel::INFO => Level::Info,
        TraceLevel::WARN => Level::Warn,
        TraceLevel::ERROR => Level::Error,
    }
}

/// An event's message, then its other fields as `key=value`.
#[derive(Default)]
struct Message {
    text: String,
    fields: String,
}

impl Visit for Message {
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        if field.name() == "message" {
            let _ = write!(self.text, "{value:?}");
        } else {
            let _ = write!(self.fields, " {}={value:?}", field.name());
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.text.push_str(value);
        } else {
            let _ = write!(self.fields, " {}={value}", field.name());
        }
    }
}

/// The line `event` makes, if it is one the site keeps.
pub fn line_of(event: &Event<'_>, at_ms: i64) -> Option<Line> {
    let level = level_of(event.metadata().level());
    if level < Level::Info {
        return None;
    }
    let mut message = Message::default();
    event.record(&mut message);
    Some(Line {
        at_ms,
        level,
        source: PLATFORM.into(),
        target: event.metadata().target().to_string(),
        message: format!("{}{}", message.text, message.fields),
    })
}

impl<S: Subscriber> Layer<S> for StoreLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        if let Some(line) = line_of(event, now_ms()) {
            let _ = self.tx.send(line);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tracing_subscriber::layer::SubscriberExt;

    use super::*;
    use crate::logs::Search;
    use crate::store::Store;

    /// **An INFO line and above is stored under the platform's source**,
    /// its fields after its message; DEBUG is not.
    #[test]
    fn platform_events_become_stored_lines() {
        let logs = Logs::new(Arc::new(Store::in_memory().unwrap()));
        let subscriber = tracing_subscriber::registry().with(StoreLayer::new(logs.clone()));
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(app = "recipes", "publish: live");
            tracing::debug!("not kept");
            tracing::warn!("check refused");
        });
        let mut found = Vec::new();
        for _ in 0..100 {
            found = logs
                .search(&Search {
                    limit: 10,
                    ..Search::default()
                })
                .unwrap()
                .lines;
            if found.len() == 2 {
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(found.len(), 2, "{found:?}");
        assert!(found[0].contains(" INFO yenbot "), "{}", found[0]);
        assert!(
            found[0].ends_with("publish: live app=recipes"),
            "{}",
            found[0]
        );
        assert!(found[1].contains(" WARN yenbot "));
    }
}
