//! The gateway's bus (`candle/docs/mcp_file_tools.md` §5): yenbot connects
//! with its service key, subscribes to `repo/yenbot`, and is woken by every
//! push Zen makes to the repository. The bus keeps nothing, so each
//! (re)connection is itself a wake: whatever was pushed while the socket was
//! down is fetched then.

use std::time::Duration;

use anyhow::{bail, Result};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;

/// The topic every push to the repository is announced on.
pub const TOPIC: &str = "repo/yenbot";

/// The longest wait between reconnections.
const MAX_BACKOFF: Duration = Duration::from_secs(30);

/// Why the site was woken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Wake {
    /// The socket (re)connected.
    Connected,
    /// A push to `branch` at `commit`.
    Pushed { branch: String, commit: String },
}

/// What one frame from the bus means: a push to the repository, a refusal
/// (logged), or nothing to act on.
pub fn wake_of(frame: &str) -> Option<Wake> {
    let v: Value = serde_json::from_str(frame).ok()?;
    match v["op"].as_str()? {
        "message" if v["topic"] == TOPIC => Some(Wake::Pushed {
            branch: v["payload"]["branch"].as_str().unwrap_or_default().into(),
            commit: v["payload"]["commit"].as_str().unwrap_or_default().into(),
        }),
        "refused" => {
            tracing::error!(topic = %v["topic"], why = %v["why"], "bus: refused");
            None
        }
        _ => None,
    }
}

/// The first wait before reconnecting, and the wait once a connection has
/// subscribed.
const FIRST_BACKOFF: Duration = Duration::from_secs(1);

/// One connection: subscribe, then send a wake for it and for each push,
/// until the socket ends. A connection that subscribed puts `backoff` back to
/// [`FIRST_BACKOFF`], so a bus that restarts is rejoined at once.
async fn connection(
    url: &str,
    key: &str,
    wakes: &Sender<Wake>,
    backoff: &mut Duration,
) -> Result<()> {
    let mut req = url.into_client_request()?;
    req.headers_mut()
        .insert("authorization", format!("Bearer {key}").parse()?);
    let (mut ws, _) = connect_async(req).await?;
    ws.send(Message::Text(
        json!({"op": "subscribe", "topics": [TOPIC]}).to_string(),
    ))
    .await?;
    tracing::info!(url, "bus: subscribed to {TOPIC}");
    *backoff = FIRST_BACKOFF;
    wakes.send(Wake::Connected).await?;
    while let Some(frame) = ws.next().await {
        match frame? {
            Message::Text(text) => {
                if let Some(wake) = wake_of(&text) {
                    wakes.send(wake).await?;
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
    bail!("the bus closed the socket")
}

/// Keep a connection to the bus at `url`, reconnecting with backoff, waking
/// `wakes` on each connection and each push.
pub async fn listen(url: String, key: String, wakes: Sender<Wake>) {
    let mut backoff = FIRST_BACKOFF;
    loop {
        if let Err(e) = connection(&url, &key, &wakes, &mut backoff).await {
            tracing::warn!(url, "bus: {e}; reconnecting in {}s", backoff.as_secs());
        }
        if wakes.is_closed() {
            return;
        }
        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(MAX_BACKOFF);
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use axum::extract::ws::{Message as AxMessage, WebSocketUpgrade};
    use axum::http::HeaderMap;
    use axum::routing::get;
    use axum::Router;
    use tokio::sync::mpsc;

    use super::*;

    #[test]
    fn a_push_to_the_repository_is_a_wake_and_nothing_else_is() {
        assert_eq!(
            wake_of(
                r#"{"op":"message","topic":"repo/yenbot","payload":{"branch":"main","commit":"4f1c"},"at_ms":1}"#
            ),
            Some(Wake::Pushed {
                branch: "main".into(),
                commit: "4f1c".into()
            })
        );
        assert_eq!(
            wake_of(r#"{"op":"message","topic":"repo/candle","payload":{}}"#),
            None
        );
        assert_eq!(
            wake_of(r#"{"op":"refused","topic":"repo/yenbot","why":"no"}"#),
            None
        );
        assert_eq!(wake_of("not json"), None);
    }

    /// A bus that checks the key and the subscription, then announces one
    /// push.
    async fn bus() -> SocketAddr {
        let app = Router::new().route(
            "/gateway/events",
            get(|headers: HeaderMap, ws: WebSocketUpgrade| async move {
                let keyed = headers.get("authorization").and_then(|v| v.to_str().ok())
                    == Some("Bearer k3y");
                ws.on_upgrade(move |mut socket| async move {
                    let Some(Ok(AxMessage::Text(sub))) = socket.recv().await else {
                        return;
                    };
                    let asked: Value = serde_json::from_str(&sub).unwrap();
                    if keyed && asked["topics"] == json!(["repo/yenbot"]) {
                        let push = json!({"op": "message", "topic": "repo/yenbot",
                                          "payload": {"branch": "main", "commit": "4f1c"}, "at_ms": 1});
                        let _ = socket.send(AxMessage::Text(push.to_string())).await;
                    }
                })
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        addr
    }

    /// **The site subscribes with its key and is woken on connecting and on
    /// each push.**
    #[tokio::test]
    async fn the_site_is_woken_on_connecting_and_on_a_push() {
        let addr = bus().await;
        let (tx, mut rx) = mpsc::channel(8);
        tokio::spawn(listen(
            format!("ws://{addr}/gateway/events"),
            "k3y".into(),
            tx,
        ));
        let within = Duration::from_secs(5);
        assert_eq!(
            tokio::time::timeout(within, rx.recv()).await.unwrap(),
            Some(Wake::Connected)
        );
        assert_eq!(
            tokio::time::timeout(within, rx.recv()).await.unwrap(),
            Some(Wake::Pushed {
                branch: "main".into(),
                commit: "4f1c".into()
            })
        );
    }
}
