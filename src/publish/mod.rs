//! Publishing (`candle/docs/yenbot_app_host.md` §5.3, §5.4): an edit is a
//! commit on `main`, let through by the gate ([`crate::gate`]), and the site
//! makes it live by fast-forwarding its live checkout ([`live`]) — woken by
//! the gateway's bus on every push ([`bus`]), and every [`POLL`] for a push
//! made outside Zen. Each move is told to the open pages ([`Reloads`]),
//! which reload themselves.

pub mod bus;
pub mod live;

use std::path::PathBuf;
use std::time::Duration;

use tokio::sync::{broadcast, mpsc};

use bus::Wake;
use live::{fast_forward, Moved};

/// How often the live checkout is fetched without a wake.
pub const POLL: Duration = Duration::from_secs(60);

/// The one branch the site serves.
pub const BRANCH: &str = "main";

/// The open pages' reload signal: the commit the site now serves.
#[derive(Clone)]
pub struct Reloads {
    tx: broadcast::Sender<String>,
}

impl Reloads {
    pub fn new() -> Reloads {
        Reloads {
            tx: broadcast::channel(16).0,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.tx.subscribe()
    }

    /// Tell every open page the site now serves `commit`.
    pub fn moved(&self, commit: &str) {
        // No page open is no one to tell.
        let _ = self.tx.send(commit.to_string());
    }
}

impl Default for Reloads {
    fn default() -> Reloads {
        Reloads::new()
    }
}

/// Why `wake` is a reason to fetch; `None` for a push to a branch the site
/// does not serve.
fn reason(wake: Wake) -> Option<String> {
    match wake {
        Wake::Pushed { branch, commit } => (branch == BRANCH).then(|| format!("push {commit}")),
        Wake::Connected => Some("the bus connected".to_string()),
    }
}

/// Follow `origin/main` into `live` for as long as the site runs: on each
/// wake from the bus at `url` (with `key`), and every [`POLL`]; and answer
/// the gate's asks against `live`'s repository.
pub fn start(live: PathBuf, url: String, key: String, reloads: Reloads) {
    let (tx, mut rx) = mpsc::channel(32);
    tokio::spawn(bus::listen(url, key, live.clone(), tx));
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(POLL);
        loop {
            // A bus listener that has ended disables its branch; the minute's
            // look goes on.
            let why = tokio::select! {
                Some(wake) = rx.recv() => match reason(wake) {
                    Some(why) => why,
                    None => continue,
                },
                _ = tick.tick() => "the minute's look".to_string(),
            };
            // One fetch covers every wake already queued behind this one.
            while rx.try_recv().is_ok() {}
            let at = live.clone();
            match tokio::task::spawn_blocking(move || fast_forward(&at)).await {
                Ok(Ok(Moved::To { from, to })) => {
                    tracing::info!(from, to, why, "publish: live");
                    reloads.moved(&to);
                }
                Ok(Ok(Moved::Unchanged)) => {}
                Ok(Err(e)) => tracing::error!(why, "publish: {e:#}"),
                Err(e) => tracing::error!(why, "publish: panicked: {e}"),
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn every_open_page_hears_a_move() {
        let reloads = Reloads::new();
        let (mut a, mut b) = (reloads.subscribe(), reloads.subscribe());
        reloads.moved("4f1c");
        assert_eq!(a.recv().await.unwrap(), "4f1c");
        assert_eq!(b.recv().await.unwrap(), "4f1c");
    }

    /// **Only a push to main, or a connection, is a reason to fetch.**
    #[test]
    fn a_push_to_another_branch_is_no_reason_to_fetch() {
        let push = |branch: &str| Wake::Pushed {
            branch: branch.into(),
            commit: "4f1c".into(),
        };
        assert_eq!(reason(push("main")).as_deref(), Some("push 4f1c"));
        assert_eq!(reason(push("zen/work")), None);
        assert_eq!(
            reason(Wake::Connected).as_deref(),
            Some("the bus connected")
        );
    }
}
