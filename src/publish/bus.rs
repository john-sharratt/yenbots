//! The gateway's bus (`candle/docs/mcp_file_tools.md` §5): yenbot connects
//! with its service key and subscribes to two topics.
//!
//! - **`repo/yenbot`**: every push Zen makes to the repository wakes the
//!   publisher. The bus keeps nothing, so each (re)connection is itself a
//!   wake: whatever was pushed while the socket was down is fetched then.
//! - **`gate/yenbot`**: before Zen moves `main` it asks whether a candidate
//!   may land; the gate ([`crate::gate`]) answers on
//!   `gate/yenbot/result/<id>`. Each ask is answered off the socket's own
//!   task, so a slow fetch holds up no other message. An ask that arrives
//!   while the socket is down is never seen; Zen, hearing nothing, refuses
//!   the push.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{bail, Result};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::sync::mpsc::{self, Sender};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;

use crate::gate::answer;
use crate::gate::ask::{Answer, Ask};

/// The topic every push to the repository is announced on.
pub const REPO: &str = "repo/yenbot";

/// The topic the gate is asked on.
pub const GATE: &str = "gate/yenbot";

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

/// What a frame from the bus is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Heard {
    Wake(Wake),
    Ask(Ask),
}

/// What one frame from the bus means: a push, an ask of the gate, a refusal
/// (logged), or nothing to act on.
pub fn heard_of(frame: &str) -> Option<Heard> {
    let v: Value = serde_json::from_str(frame).ok()?;
    match v["op"].as_str()? {
        "message" if v["topic"] == REPO => Some(Heard::Wake(Wake::Pushed {
            branch: v["payload"]["branch"].as_str().unwrap_or_default().into(),
            commit: v["payload"]["commit"].as_str().unwrap_or_default().into(),
        })),
        "message" if v["topic"] == GATE => match Ask::of(&v["payload"]) {
            Some(ask) => Some(Heard::Ask(ask)),
            None => {
                tracing::error!(payload = %v["payload"], "gate: an ask with no id cannot be answered");
                None
            }
        },
        "refused" => {
            tracing::error!(topic = %v["topic"], why = %v["why"], "bus: refused");
            None
        }
        _ => None,
    }
}

/// The frame that publishes `answer` to `ask`.
fn reply(ask: &Ask, answer: &Answer) -> String {
    json!({"op": "publish", "topic": ask.reply_topic(), "payload": answer.payload()}).to_string()
}

/// Answer `ask` against the repository at `repo`, and send the reply to
/// `out`, the socket's writer.
fn answer_in_turn(repo: Arc<PathBuf>, ask: Ask, out: Sender<String>) {
    tokio::spawn(async move {
        let started = Instant::now();
        let asked = ask.clone();
        let answered = tokio::task::spawn_blocking(move || answer(&repo, &asked))
            .await
            .unwrap_or_else(|e| Answer::Refused(format!("the gate failed: {e}")));
        let ms = started.elapsed().as_millis();
        match &answered {
            Answer::Ok => {
                tracing::info!(id = %ask.id, commit = ?ask.commit, branch = %ask.branch, ms, "gate: ok")
            }
            Answer::Refused(why) => {
                tracing::warn!(id = %ask.id, commit = ?ask.commit, branch = %ask.branch, ms, "gate: refused: {why}")
            }
        }
        // A socket that closed meanwhile has no one to tell; Zen refuses the
        // push when no answer comes.
        let _ = out.send(reply(&ask, &answered)).await;
    });
}

/// The first wait before reconnecting, and the wait once a connection has
/// subscribed.
const FIRST_BACKOFF: Duration = Duration::from_secs(1);

/// One connection: subscribe, then wake the publisher for it and for each
/// push, and answer each ask, until the socket ends. A connection that
/// subscribed puts `backoff` back to [`FIRST_BACKOFF`], so a bus that
/// restarts is rejoined at once.
async fn connection(
    url: &str,
    key: &str,
    repo: &Arc<PathBuf>,
    wakes: &Sender<Wake>,
    backoff: &mut Duration,
) -> Result<()> {
    let mut req = url.into_client_request()?;
    req.headers_mut()
        .insert("authorization", format!("Bearer {key}").parse()?);
    let (ws, _) = connect_async(req).await?;
    let (mut sink, mut stream) = ws.split();
    sink.send(Message::Text(
        json!({"op": "subscribe", "topics": [REPO, GATE]}).to_string(),
    ))
    .await?;
    tracing::info!(url, "bus: subscribed to {REPO} and {GATE}");
    *backoff = FIRST_BACKOFF;
    wakes.send(Wake::Connected).await?;
    let (out, mut outgoing) = mpsc::channel::<String>(32);
    loop {
        tokio::select! {
            frame = stream.next() => {
                let Some(frame) = frame else { break };
                match frame? {
                    Message::Text(text) => match heard_of(&text) {
                        Some(Heard::Wake(wake)) => wakes.send(wake).await?,
                        Some(Heard::Ask(ask)) => answer_in_turn(repo.clone(), ask, out.clone()),
                        None => {}
                    },
                    Message::Close(_) => break,
                    _ => {}
                }
            }
            Some(frame) = outgoing.recv() => sink.send(Message::Text(frame)).await?,
        }
    }
    bail!("the bus closed the socket")
}

/// Keep a connection to the bus at `url`, reconnecting with backoff: waking
/// `wakes` on each connection and each push, and answering the gate's asks
/// against the repository at `repo`.
pub async fn listen(url: String, key: String, repo: PathBuf, wakes: Sender<Wake>) {
    let repo = Arc::new(repo);
    let mut backoff = FIRST_BACKOFF;
    loop {
        if let Err(e) = connection(&url, &key, &repo, &wakes, &mut backoff).await {
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
    fn a_push_is_a_wake_an_ask_is_an_ask_and_nothing_else_is_either() {
        assert_eq!(
            heard_of(
                r#"{"op":"message","topic":"repo/yenbot","payload":{"branch":"main","commit":"4f1c"},"at_ms":1}"#
            ),
            Some(Heard::Wake(Wake::Pushed {
                branch: "main".into(),
                commit: "4f1c".into()
            }))
        );
        let ask = heard_of(
            r#"{"op":"message","topic":"gate/yenbot","payload":{"id":"9","ref":"refs/heads/zen/candidate/9","commit":"4f1c","base":"a3f9","branch":"main"}}"#,
        );
        assert!(matches!(ask, Some(Heard::Ask(Ask { ref id, .. })) if id == "9"));
        assert_eq!(
            heard_of(r#"{"op":"message","topic":"gate/yenbot","payload":{}}"#),
            None
        );
        assert_eq!(
            heard_of(r#"{"op":"message","topic":"repo/candle","payload":{}}"#),
            None
        );
        assert_eq!(
            heard_of(r#"{"op":"refused","topic":"repo/yenbot","why":"no"}"#),
            None
        );
        assert_eq!(heard_of("not json"), None);
    }

    /// A bus that checks the key and the subscription, announces one push,
    /// asks the gate once, and hands back what the site publishes.
    async fn bus(published: mpsc::Sender<Value>) -> SocketAddr {
        let app = Router::new().route(
            "/gateway/events",
            get(move |headers: HeaderMap, ws: WebSocketUpgrade| {
                let published = published.clone();
                async move {
                    let keyed = headers.get("authorization").and_then(|v| v.to_str().ok()) == Some("Bearer k3y");
                    ws.on_upgrade(move |mut socket| async move {
                        let Some(Ok(AxMessage::Text(sub))) = socket.recv().await else {
                            return;
                        };
                        let asked: Value = serde_json::from_str(&sub).unwrap();
                        if !(keyed && asked["topics"] == json!(["repo/yenbot", "gate/yenbot"])) {
                            return;
                        }
                        let push = json!({"op": "message", "topic": "repo/yenbot",
                                          "payload": {"branch": "main", "commit": "4f1c"}, "at_ms": 1});
                        let _ = socket.send(AxMessage::Text(push.to_string())).await;
                        let ask = json!({"op": "message", "topic": "gate/yenbot", "at_ms": 2,
                                         "payload": {"id": "c9", "ref": "refs/heads/zen/candidate/c9", "commit": "4f1c",
                                                     "base": "a3f9", "branch": "zen/work"}});
                        let _ = socket.send(AxMessage::Text(ask.to_string())).await;
                        while let Some(Ok(AxMessage::Text(frame))) = socket.recv().await {
                            let _ = published.send(serde_json::from_str(&frame).unwrap()).await;
                        }
                    })
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        addr
    }

    /// **The site subscribes with its key, is woken on connecting and on each
    /// push, and answers the gate's ask on its reply topic.**
    #[tokio::test]
    async fn the_site_is_woken_and_answers_the_gate() {
        let (tx_pub, mut published) = mpsc::channel(8);
        let addr = bus(tx_pub).await;
        let (tx, mut rx) = mpsc::channel(8);
        let repo = tempfile::tempdir().unwrap();
        tokio::spawn(listen(
            format!("ws://{addr}/gateway/events"),
            "k3y".into(),
            repo.path().to_path_buf(),
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
        assert_eq!(
            tokio::time::timeout(within, published.recv())
                .await
                .unwrap(),
            Some(
                json!({"op": "publish", "topic": "gate/yenbot/result/c9", "payload": {"ok": true}})
            )
        );
    }
}
