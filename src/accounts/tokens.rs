//! API tokens (§6.3): made for an admin's account on the admin page, named,
//! with an expiry, shown once. Only the SHA-256 of the secret is kept, so the
//! store never holds a token that works.

use anyhow::{bail, Result};
use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::store::Store;

/// The longest a token may last.
pub const MAX_DAYS: i64 = 366;

/// The SHA-256 of `secret`, trimmed, as hex: what the store keeps of a
/// token — an API token's, or an inbound hook's.
pub fn hash(secret: &str) -> String {
    Sha256::digest(secret.trim().as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// A secret of 256 bits from the operating system, as 64 hex digits.
pub fn fresh_secret() -> Result<String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|e| anyhow::anyhow!("no randomness: {e}"))?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// One token, as the admin page lists it — never its secret.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Listed {
    pub name: String,
    pub account: String,
    pub created_ms: i64,
    pub expires_ms: i64,
}

/// Make token `name` for `account`, lasting `days`; answers its secret, the
/// only time it is shown.
pub fn create(store: &Store, name: &str, account: &str, days: i64, now: i64) -> Result<String> {
    if name.trim().is_empty() {
        bail!("a token needs a name");
    }
    if !(1..=MAX_DAYS).contains(&days) {
        bail!("a token lasts 1 to {MAX_DAYS} days");
    }
    let secret = fresh_secret()?;
    store.conn().execute(
        "INSERT INTO api_tokens (hash, name, account, created_ms, expires_ms) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![hash(&secret), name.trim(), account, now, now + days * 86_400_000],
    )?;
    Ok(secret)
}

/// The account `secret` acts as and the token's name, while it has not
/// expired.
pub fn holder(store: &Store, secret: &str, now: i64) -> Result<Option<(String, String)>> {
    Ok(store
        .conn()
        .query_row(
            "SELECT account, name FROM api_tokens WHERE hash = ?1 AND expires_ms > ?2",
            params![hash(secret), now],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?)
}

pub fn list(store: &Store) -> Result<Vec<Listed>> {
    let conn = store.conn();
    let mut stmt = conn.prepare(
        "SELECT name, account, created_ms, expires_ms FROM api_tokens ORDER BY created_ms",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(Listed {
            name: r.get(0)?,
            account: r.get(1)?,
            created_ms: r.get(2)?,
            expires_ms: r.get(3)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Revoke token `name`; whether there was one.
pub fn revoke(store: &Store, name: &str) -> Result<bool> {
    Ok(store
        .conn()
        .execute("DELETE FROM api_tokens WHERE name = ?1", params![name])?
        > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store_with(account: &str) -> Store {
        let store = Store::in_memory().unwrap();
        store
            .conn()
            .execute(
                "INSERT INTO accounts (id, provider, email, name, status, created_ms, seen_ms) \
                 VALUES (?1, 'google', 'a@b.c', 'A', 'active', 0, 0)",
                params![account],
            )
            .unwrap();
        store
    }

    /// **A token acts as its account until it expires or is revoked**, and
    /// the store holds only its hash.
    #[test]
    fn a_token_acts_until_it_expires_or_is_revoked() {
        let store = store_with("1");
        let secret = create(&store, "zen", "1", 30, 1_000).unwrap();
        assert_eq!(secret.len(), 64);
        assert_eq!(
            holder(&store, &secret, 2_000).unwrap(),
            Some(("1".into(), "zen".into()))
        );
        assert_eq!(holder(&store, "wrong", 2_000).unwrap(), None);
        let expired = 1_000 + 30 * 86_400_000;
        assert_eq!(holder(&store, &secret, expired).unwrap(), None);
        let held: String = store
            .conn()
            .query_row("SELECT hash FROM api_tokens", [], |r| r.get(0))
            .unwrap();
        assert_ne!(held, secret);
        assert_eq!(list(&store).unwrap()[0].name, "zen");
        assert!(revoke(&store, "zen").unwrap());
        assert_eq!(holder(&store, &secret, 2_000).unwrap(), None);
    }

    #[test]
    fn a_token_needs_a_name_and_a_sane_life() {
        let store = store_with("1");
        assert!(create(&store, " ", "1", 30, 0).is_err());
        assert!(create(&store, "t", "1", 0, 0).is_err());
        assert!(create(&store, "t", "1", MAX_DAYS + 1, 0).is_err());
    }
}
