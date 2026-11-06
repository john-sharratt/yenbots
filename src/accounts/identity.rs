//! Who a request is: the identity the gateway states, on the headers it sets
//! from a session it verified and strips from everything arriving from
//! outside (`candle/web/src/proxy.rs`). yenbot never sees a Google secret.
//!
//! **A known hole, accepted for now**: these headers are believed on any
//! request reaching the loopback port, as every daemon behind the gateway
//! believes them, and a Zen worker's raw socket can reach loopback
//! (`candle/docs/mcp_file_tools.md` §4.1).

use axum::http::HeaderMap;

pub const USER: &str = "x-tokera-user";
pub const PROVIDER: &str = "x-tokera-provider";
pub const EMAIL: &str = "x-tokera-email";
pub const NAME: &str = "x-tokera-name";
pub const PICTURE: &str = "x-tokera-picture";

/// A signed-in person, as the gateway names them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub provider: String,
    pub sub: String,
    pub email: String,
    pub name: String,
    pub picture: String,
}

/// The identity `headers` carry; `None` for a request nobody is signed in on.
/// A subject without a provider is no identity: the two together are the
/// account key.
pub fn identify(headers: &HeaderMap) -> Option<Identity> {
    let get = |name: &str| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .trim()
            .to_string()
    };
    let (sub, provider) = (get(USER), get(PROVIDER));
    if sub.is_empty() || provider.is_empty() {
        return None;
    }
    Some(Identity {
        provider,
        sub,
        email: get(EMAIL),
        name: get(NAME),
        picture: get(PICTURE),
    })
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    fn headers(pairs: &[(&'static str, &'static str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.insert(*k, HeaderValue::from_static(v));
        }
        h
    }

    #[test]
    fn the_gateways_headers_name_the_person() {
        let id = identify(&headers(&[
            (USER, "1043"),
            (PROVIDER, "google"),
            (EMAIL, "yen@example.com"),
            (NAME, "Yen"),
        ]))
        .unwrap();
        assert_eq!(
            id,
            Identity {
                provider: "google".into(),
                sub: "1043".into(),
                email: "yen@example.com".into(),
                name: "Yen".into(),
                picture: String::new(),
            }
        );
    }

    #[test]
    fn no_subject_or_no_provider_is_nobody() {
        assert_eq!(identify(&HeaderMap::new()), None);
        assert_eq!(identify(&headers(&[(USER, "1043")])), None);
        assert_eq!(
            identify(&headers(&[(USER, " "), (PROVIDER, "google")])),
            None
        );
    }
}
