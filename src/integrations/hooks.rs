//! Inbound hooks (§19.2): `/hooks/<app>/<name>` runs the app's
//! `workflows/hooks/<name>.js` for a service that calls in. It is reached
//! without signing in, so a call carries proof, one of:
//!
//! - **the hook's token**, made on the admin page and shown once, as
//!   `?token=` or `Authorization: Bearer`; the store keeps its hash;
//! - **a signature** of the body, `X-Yen-Signature: sha256=<hex>` — the
//!   HMAC-SHA256 keyed by the app's secret `hook-<name>` (§19.1), as a
//!   service that signs its calls (GitHub's `X-Hub-Signature-256`) sends.
//!
//! A call with neither is refused and logged.

use anyhow::Result;
use hmac::{Hmac, Mac};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use sha2::Sha256;

use crate::accounts::tokens::{fresh_secret, hash};

/// The workflow a hook runs: `hooks/<name>`, a file under `workflows/`.
pub fn workflow_of(name: &str) -> String {
    format!("hooks/{name}")
}

/// The app secret a hook's signatures are keyed by.
pub fn signing_secret(name: &str) -> String {
    format!("hook-{name}")
}

/// A hook's token, as the admin page lists it — never the token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Listed {
    pub hook: String,
    pub created_ms: i64,
}

/// Make a token for `app`'s hook `hook`, replacing the one it had; answers
/// it, the only time it is shown.
pub fn make(conn: &Connection, app: &str, hook: &str, now: i64) -> Result<String> {
    let token = format!("yh_{}", fresh_secret()?);
    conn.execute(
        "INSERT INTO hook_tokens (app, hook, hash, created_ms) VALUES (?1, ?2, ?3, ?4) \
         ON CONFLICT (app, hook) DO UPDATE SET hash = ?3, created_ms = ?4",
        params![app, hook, hash(&token), now],
    )?;
    Ok(token)
}

/// Revoke the token of `app`'s hook `hook`; whether it had one.
pub fn revoke(conn: &Connection, app: &str, hook: &str) -> Result<bool> {
    Ok(conn.execute(
        "DELETE FROM hook_tokens WHERE app = ?1 AND hook = ?2",
        params![app, hook],
    )? > 0)
}

/// The hooks of `app` that have a token.
pub fn list(conn: &Connection, app: &str) -> Result<Vec<Listed>> {
    let mut stmt =
        conn.prepare("SELECT hook, created_ms FROM hook_tokens WHERE app = ?1 ORDER BY hook")?;
    let rows = stmt.query_map(params![app], |r| {
        Ok(Listed {
            hook: r.get(0)?,
            created_ms: r.get(1)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Whether `token` is the token of `app`'s hook `hook`.
pub fn token_holds(conn: &Connection, app: &str, hook: &str, token: &str) -> Result<bool> {
    let held: Option<String> = conn
        .query_row(
            "SELECT hash FROM hook_tokens WHERE app = ?1 AND hook = ?2",
            params![app, hook],
            |r| r.get(0),
        )
        .optional()?;
    Ok(held.is_some_and(|h| h == hash(token)))
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok())
        .collect()
}

/// Whether `header` — `sha256=<hex>` — is the HMAC-SHA256 of `body` keyed
/// by `key`, compared in constant time.
pub fn signature_holds(key: &[u8], body: &[u8], header: &str) -> bool {
    let Some(sig) = header.trim().strip_prefix("sha256=").and_then(unhex) else {
        return false;
    };
    let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(key) else {
        return false;
    };
    mac.update(body);
    mac.verify_slice(&sig).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;

    /// **A hook's token works until it is replaced or revoked**, for its
    /// own hook and app only, and the store keeps only its hash.
    #[test]
    fn a_hooks_token_works_until_replaced_or_revoked() {
        let store = Store::in_memory().unwrap();
        let conn = store.conn();
        let first = make(&conn, "chores", "done", 5).unwrap();
        assert_eq!((first.len(), &first[..3]), (67, "yh_"));
        assert!(token_holds(&conn, "chores", "done", &first).unwrap());
        assert!(!token_holds(&conn, "chores", "other", &first).unwrap());
        assert!(!token_holds(&conn, "recipes", "done", &first).unwrap());
        let held: String = conn
            .query_row("SELECT hash FROM hook_tokens", [], |r| r.get(0))
            .unwrap();
        assert_eq!(held, hash(&first));

        let second = make(&conn, "chores", "done", 9).unwrap();
        assert!(!token_holds(&conn, "chores", "done", &first).unwrap());
        assert!(token_holds(&conn, "chores", "done", &second).unwrap());
        assert_eq!(
            list(&conn, "chores").unwrap(),
            [Listed {
                hook: "done".into(),
                created_ms: 9
            }]
        );
        assert!(revoke(&conn, "chores", "done").unwrap());
        assert!(!token_holds(&conn, "chores", "done", &second).unwrap());
        assert!(!revoke(&conn, "chores", "done").unwrap());
    }

    /// **A signature is the body's HMAC-SHA256** — RFC 4231's second case —
    /// and anything else is refused.
    #[test]
    fn a_signature_is_the_bodys_hmac() {
        let body = b"what do ya want for nothing?";
        let good = "sha256=5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843";
        assert!(signature_holds(b"Jefe", body, good));
        assert!(!signature_holds(b"Jeff", body, good));
        assert!(!signature_holds(b"Jefe", b"what do ya want?", good));
        assert!(!signature_holds(b"Jefe", body, &good[7..]), "no prefix");
        assert!(!signature_holds(b"Jefe", body, "sha256=zz"));
        assert!(!signature_holds(b"Jefe", body, "sha256=5bd"));
        assert!(!signature_holds(b"Jefe", body, ""));
    }

    #[test]
    fn a_hook_is_a_workflow_with_its_own_secret() {
        assert_eq!(workflow_of("form"), "hooks/form");
        assert_eq!(signing_secret("form"), "hook-form");
    }
}
