//! The site's push key (VAPID, RFC 8292): one P-256 key, made at first start
//! and kept in `push_keys`. A device subscribes with its public half; each
//! push is sent with a short token it signs, which the push service checks
//! against the key the device subscribed with.

use anyhow::{anyhow, Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use p256::ecdsa::signature::Signer;
use p256::ecdsa::{Signature, SigningKey};
use p256::elliptic_curve::sec1::ToEncodedPoint;
use p256::SecretKey;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};

/// Who the push services may reach about this site: its address, so no
/// person's email leaves it.
pub const SUBJECT: &str = "https://yenbot.com.au";

/// How long a token is good for: the push services take at most a day.
const TOKEN_SECS: i64 = 12 * 3600;

#[derive(Clone)]
pub struct Vapid {
    key: SecretKey,
}

impl Vapid {
    /// The site's key as `key`, for tests.
    #[cfg(test)]
    pub fn from_key(key: SecretKey) -> Vapid {
        Vapid { key }
    }

    /// The site's key, made the first time it is asked for.
    pub fn load_or_make(conn: &Connection, now: i64) -> Result<Vapid> {
        let held: Option<String> = conn
            .query_row("SELECT private FROM push_keys WHERE id = 1", [], |r| {
                r.get(0)
            })
            .optional()?;
        if let Some(b64) = held {
            let bytes = URL_SAFE_NO_PAD
                .decode(b64)
                .context("the push key is not base64")?;
            let key = SecretKey::from_slice(&bytes).context("the push key is not a P-256 key")?;
            return Ok(Vapid { key });
        }
        let mut seed = [0u8; 32];
        let key = loop {
            getrandom::fill(&mut seed).map_err(|e| anyhow!("no randomness: {e}"))?;
            if let Ok(k) = SecretKey::from_slice(&seed) {
                break k;
            }
        };
        let v = Vapid { key };
        conn.execute(
            "INSERT INTO push_keys (id, private, public, created_ms) VALUES (1, ?1, ?2, ?3)",
            params![URL_SAFE_NO_PAD.encode(v.key.to_bytes()), v.public(), now],
        )?;
        Ok(v)
    }

    /// The public half, uncompressed and base64url: what a device subscribes
    /// with, as `applicationServerKey`.
    pub fn public(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.key.public_key().to_encoded_point(false).as_bytes())
    }

    /// The token for a push to the service at `origin`
    /// (`https://fcm.googleapis.com`), good until `now` plus twelve hours.
    pub fn token(&self, origin: &str, now_ms: i64) -> String {
        let part = |v: Value| URL_SAFE_NO_PAD.encode(v.to_string());
        let head = part(json!({"typ": "JWT", "alg": "ES256"}));
        let claims =
            part(json!({"aud": origin, "exp": now_ms / 1000 + TOKEN_SECS, "sub": SUBJECT}));
        let signed = format!("{head}.{claims}");
        let sig: Signature = SigningKey::from(&self.key).sign(signed.as_bytes());
        format!("{signed}.{}", URL_SAFE_NO_PAD.encode(sig.to_bytes()))
    }

    /// The `Authorization` header a push carries.
    pub fn authorization(&self, origin: &str, now_ms: i64) -> String {
        format!(
            "vapid t={}, k={}",
            self.token(origin, now_ms),
            self.public()
        )
    }
}

/// The origin of a push endpoint: its scheme and host, the token's audience.
pub fn origin_of(endpoint: &str) -> Option<String> {
    let rest = endpoint.strip_prefix("https://")?;
    let host = rest.split('/').next().filter(|h| !h.is_empty())?;
    Some(format!("https://{host}"))
}

#[cfg(test)]
mod tests {
    use p256::ecdsa::signature::Verifier;
    use p256::ecdsa::VerifyingKey;

    use super::*;
    use crate::store::Store;

    /// **A token says who it is for, until when and who sent it, and the
    /// site's key signs it.**
    #[test]
    fn a_token_is_signed_by_the_sites_key() {
        let v = Vapid::from_key(SecretKey::from_slice(&[7u8; 32]).unwrap());
        let t = v.token("https://fcm.googleapis.com", 1_000_000);
        let parts: Vec<&str> = t.split('.').collect();
        assert_eq!(parts.len(), 3);
        let text = |p: &str| String::from_utf8(URL_SAFE_NO_PAD.decode(p).unwrap()).unwrap();
        assert_eq!(text(parts[0]), r#"{"alg":"ES256","typ":"JWT"}"#);
        assert_eq!(
            text(parts[1]),
            r#"{"aud":"https://fcm.googleapis.com","exp":44200,"sub":"https://yenbot.com.au"}"#
        );
        let sig = Signature::from_slice(&URL_SAFE_NO_PAD.decode(parts[2]).unwrap()).unwrap();
        let key = VerifyingKey::from(&SigningKey::from(&v.key));
        assert!(key
            .verify(format!("{}.{}", parts[0], parts[1]).as_bytes(), &sig)
            .is_ok());
        assert_eq!(
            t,
            v.token("https://fcm.googleapis.com", 1_000_000),
            "deterministic"
        );
        assert!(v.authorization("https://x", 0).starts_with("vapid t="));
        assert_eq!(v.public().len(), 87, "65 bytes, base64url");
    }

    #[test]
    fn the_key_is_made_once_and_kept() {
        let store = Store::in_memory().unwrap();
        let a = Vapid::load_or_make(&store.conn(), 1).unwrap();
        let b = Vapid::load_or_make(&store.conn(), 2).unwrap();
        assert_eq!(a.public(), b.public());
    }

    #[test]
    fn an_endpoints_origin_is_its_host() {
        assert_eq!(
            origin_of("https://fcm.googleapis.com/fcm/send/abc").as_deref(),
            Some("https://fcm.googleapis.com")
        );
        assert_eq!(
            origin_of("https://web.push.apple.com/QGs").as_deref(),
            Some("https://web.push.apple.com")
        );
        assert_eq!(origin_of("http://x/y"), None);
        assert_eq!(origin_of("https:///y"), None);
    }
}
