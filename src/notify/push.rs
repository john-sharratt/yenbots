//! Sending a push (§11.1): each message is encrypted to its device and
//! posted to the device's push service by one sender thread, so whatever
//! asked for it — a workflow, a feedback note — never waits on the network.
//! A device its push service says is gone (404, 410) is forgotten.

use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use p256::SecretKey;
use ureq::{Agent, AgentBuilder, Error as Refused};

use super::devices::{gone, sent, Subscription};
use super::encrypt::{encrypt, Seal};
use super::vapid::{origin_of, Vapid};
use crate::accounts::now_ms;
use crate::store::Store;

/// How long a push service keeps a message for a device that is off.
pub const TTL_SECS: u32 = 24 * 3600;

/// One message for one device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Push {
    pub to: Subscription,
    /// What the service worker is handed: JSON.
    pub payload: Vec<u8>,
}

/// A request to a push service.
pub struct Post<'a> {
    pub endpoint: &'a str,
    pub headers: Vec<(&'static str, String)>,
    pub body: Vec<u8>,
}

/// What carries a request to a push service and answers its status.
pub trait Transport: Send + Sync {
    fn post(&self, post: &Post<'_>) -> Result<u16>;
}

/// HTTPS, through `ureq`.
pub struct Https {
    agent: Agent,
}

impl Https {
    pub fn new() -> Https {
        Https {
            agent: AgentBuilder::new().timeout(Duration::from_secs(15)).build(),
        }
    }
}

impl Default for Https {
    fn default() -> Https {
        Https::new()
    }
}

impl Transport for Https {
    fn post(&self, post: &Post<'_>) -> Result<u16> {
        let mut req = self.agent.post(post.endpoint);
        for (k, v) in &post.headers {
            req = req.set(k, v);
        }
        match req.send_bytes(&post.body) {
            Ok(res) => Ok(res.status()),
            Err(Refused::Status(code, _)) => Ok(code),
            Err(e) => Err(anyhow!("{e}")),
        }
    }
}

/// Where pushes are handed to the sender.
#[derive(Clone)]
pub struct Pusher {
    tx: Sender<Push>,
}

impl Pusher {
    /// A pusher and the queue it hands pushes to.
    pub fn channel() -> (Pusher, Receiver<Push>) {
        let (tx, rx) = channel();
        (Pusher { tx }, rx)
    }

    /// A pusher whose sender thread is started, posting with `transport`.
    pub fn start(store: Arc<Store>, vapid: Vapid, transport: Arc<dyn Transport>) -> Pusher {
        let (pusher, rx) = Pusher::channel();
        thread::Builder::new()
            .name("yenbot-push".into())
            .spawn(move || {
                for push in rx {
                    if let Err(e) = send(&store, &vapid, transport.as_ref(), &push, now_ms()) {
                        tracing::warn!(target: "yenbot::notify", "a push was not sent: {e:#}");
                    }
                }
            })
            .expect("the push thread starts");
        pusher
    }

    /// Queue `push`.
    pub fn push(&self, push: Push) {
        // The sender is the site's own thread; with it gone the site is
        // stopping, and the inbox holds the message.
        let _ = self.tx.send(push);
    }
}

/// A key for one message, made fresh.
fn fresh_key() -> Result<SecretKey> {
    let mut seed = [0u8; 32];
    loop {
        getrandom::fill(&mut seed).map_err(|e| anyhow!("no randomness: {e}"))?;
        if let Ok(k) = SecretKey::from_slice(&seed) {
            return Ok(k);
        }
    }
}

/// The request that carries `push`, encrypted with `ours` and `salt`.
pub fn post_of<'a>(
    vapid: &Vapid,
    push: &'a Push,
    ours: &SecretKey,
    salt: [u8; 16],
    now: i64,
) -> Result<Post<'a>> {
    let endpoint = push.to.endpoint.as_str();
    let origin =
        origin_of(endpoint).ok_or_else(|| anyhow!("`{endpoint}` is not an https endpoint"))?;
    let device_key = URL_SAFE_NO_PAD
        .decode(push.to.keys.p256dh.trim_end_matches('='))
        .map_err(|_| anyhow!("the device's key is not base64url"))?;
    let auth = URL_SAFE_NO_PAD
        .decode(push.to.keys.auth.trim_end_matches('='))
        .map_err(|_| anyhow!("the device's auth secret is not base64url"))?;
    let body = encrypt(
        &Seal {
            device_key: &device_key,
            auth: &auth,
            ours,
            salt,
        },
        &push.payload,
    )?;
    Ok(Post {
        endpoint,
        headers: vec![
            ("Authorization", vapid.authorization(&origin, now)),
            ("Content-Encoding", "aes128gcm".into()),
            ("Content-Type", "application/octet-stream".into()),
            ("TTL", TTL_SECS.to_string()),
            ("Urgency", "normal".into()),
        ],
        body,
    })
}

/// Send `push` now, and keep what its push service said.
pub fn send(
    store: &Store,
    vapid: &Vapid,
    transport: &dyn Transport,
    push: &Push,
    now: i64,
) -> Result<()> {
    let mut salt = [0u8; 16];
    getrandom::fill(&mut salt).map_err(|e| anyhow!("no randomness: {e}"))?;
    let post = post_of(vapid, push, &fresh_key()?, salt, now)?;
    let status = transport.post(&post)?;
    match status {
        200..=299 => sent(&store.conn(), &push.to.endpoint, now),
        404 | 410 => {
            tracing::info!(target: "yenbot::notify", "a device is gone; forgotten");
            gone(&store.conn(), &push.to.endpoint)
        }
        other => Err(anyhow!("its push service answered {other}")),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use p256::elliptic_curve::sec1::ToEncodedPoint;

    use super::*;
    use crate::notify::devices::{of, subscribe, Keys};

    /// What a push service was sent: the endpoint, and the headers.
    type Seen = (String, Vec<(&'static str, String)>);

    /// A push service that answers `status` and keeps what it was sent.
    struct Service {
        status: u16,
        seen: Mutex<Vec<Seen>>,
    }

    impl Transport for Service {
        fn post(&self, post: &Post<'_>) -> Result<u16> {
            self.seen
                .lock()
                .unwrap()
                .push((post.endpoint.to_string(), post.headers.clone()));
            Ok(self.status)
        }
    }

    fn device() -> Subscription {
        let key = SecretKey::from_slice(&[9u8; 32]).unwrap();
        Subscription {
            endpoint: "https://fcm.googleapis.com/fcm/send/abc".into(),
            keys: Keys {
                p256dh: URL_SAFE_NO_PAD.encode(key.public_key().to_encoded_point(false).as_bytes()),
                auth: URL_SAFE_NO_PAD.encode([3u8; 16]),
            },
        }
    }

    /// **A push carries the site's token for its service, says it is
    /// encrypted, and lasts a day.**
    #[test]
    fn a_push_is_posted_as_web_push_says() {
        let vapid = Vapid::from_key(SecretKey::from_slice(&[7u8; 32]).unwrap());
        let push = Push {
            to: device(),
            payload: b"{\"title\":\"hi\"}".to_vec(),
        };
        let post = post_of(
            &vapid,
            &push,
            &SecretKey::from_slice(&[5u8; 32]).unwrap(),
            [1; 16],
            0,
        )
        .unwrap();
        assert_eq!(post.endpoint, "https://fcm.googleapis.com/fcm/send/abc");
        let names: Vec<&str> = post.headers.iter().map(|h| h.0).collect();
        assert_eq!(
            names,
            [
                "Authorization",
                "Content-Encoding",
                "Content-Type",
                "TTL",
                "Urgency"
            ]
        );
        assert_eq!(
            post.headers[0].1,
            vapid.authorization("https://fcm.googleapis.com", 0)
        );
        assert_eq!(post.headers[3].1, "86400");
        // Salt, record size, key length, key, then the payload and its tag.
        assert_eq!(&post.body[..16], &[1u8; 16]);
        assert_eq!(&post.body[16..21], &[0, 0, 16, 0, 65]);
        assert_eq!(post.body.len(), 21 + 65 + push.payload.len() + 1 + 16);
    }

    /// **A device its push service says is gone is forgotten; one that took
    /// it is marked sent.**
    #[test]
    fn a_gone_device_is_forgotten() {
        let store = Store::in_memory().unwrap();
        store
            .conn()
            .execute(
                "INSERT INTO accounts (id, provider, email, name, status, created_ms, seen_ms) \
                 VALUES ('1', 'google', 'a', 'A', 'active', 0, 0)",
                [],
            )
            .unwrap();
        subscribe(&store.conn(), "1", &device(), 0).unwrap();
        let vapid = Vapid::from_key(SecretKey::from_slice(&[7u8; 32]).unwrap());
        let push = Push {
            to: device(),
            payload: b"{}".to_vec(),
        };
        let ok = Service {
            status: 201,
            seen: Mutex::default(),
        };
        send(&store, &vapid, &ok, &push, 5).unwrap();
        let at: Option<i64> = store
            .conn()
            .query_row("SELECT sent_ms FROM push_subscriptions", [], |r| r.get(0))
            .unwrap();
        assert_eq!(at, Some(5));
        let refused = Service {
            status: 500,
            seen: Mutex::default(),
        };
        assert!(send(&store, &vapid, &refused, &push, 6).is_err());
        assert_eq!(of(&store.conn(), "1").unwrap().len(), 1);
        let gone = Service {
            status: 410,
            seen: Mutex::default(),
        };
        send(&store, &vapid, &gone, &push, 7).unwrap();
        assert!(of(&store.conn(), "1").unwrap().is_empty());
        assert_eq!(gone.seen.lock().unwrap()[0].0, push.to.endpoint);
    }
}
