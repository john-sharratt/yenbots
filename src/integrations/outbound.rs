//! A workflow's calls out (§19.1): `ctx.fetch(url, {method, headers, body})`
//! to a host its manifest's `fetch:` lists — exactly that host, over HTTPS,
//! `GET`, `POST`, `PUT` or `DELETE`. A redirect is answered as it is, never
//! followed, so a listed host cannot send the call on to one that is not.
//! A browser never calls out through the site: only a workflow does.

use std::collections::BTreeMap;
use std::io::Read;
use std::net::IpAddr;
use std::time::Duration;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use ureq::{Agent, AgentBuilder, Error as Answered};

/// The methods a call may make.
pub const METHODS: [&str; 4] = ["GET", "POST", "PUT", "DELETE"];

/// The headers the site sets itself.
const OWN_HEADERS: [&str; 4] = ["host", "content-length", "transfer-encoding", "connection"];

/// The most of an answer that is read.
pub const MAX_ANSWER: u64 = 5 << 20;

/// The longest a call may take.
pub const TIMEOUT: Duration = Duration::from_secs(30);

/// A call to make, as `ctx.fetch` asks it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Call {
    pub url: String,
    #[serde(default = "get")]
    pub method: String,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub body: Option<String>,
}

fn get() -> String {
    "GET".to_string()
}

/// What the service answered: its status, headers (lowercase names) and
/// body as text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Reply {
    pub status: u16,
    pub headers: BTreeMap<String, String>,
    pub text: String,
}

/// Whatever makes the calls: the internet, or a test's fake.
pub trait Web: Send + Sync {
    fn send(&self, call: &Call) -> Result<Reply>;
}

/// Whether `host` is one a manifest may list: a domain name of lowercase
/// labels, with a dot, not an address and not this machine.
pub fn is_host(host: &str) -> bool {
    let label = |l: &str| {
        !l.is_empty()
            && l.len() <= 63
            && !l.starts_with('-')
            && !l.ends_with('-')
            && l.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    };
    host.len() <= 253
        && host.contains('.')
        && host.parse::<IpAddr>().is_err()
        && !host.ends_with(".localhost")
        && !host.ends_with(".local")
        && host.split('.').all(label)
}

/// The host `url` calls, when it is an `https://` address with no
/// credentials in it.
pub fn host_of(url: &str) -> Option<&str> {
    let rest = url.strip_prefix("https://")?;
    let authority = rest.split(['/', '?', '#']).next()?;
    if authority.contains('@') {
        return None;
    }
    let host = match authority.rsplit_once(':') {
        Some((h, port)) if !port.is_empty() && port.chars().all(|c| c.is_ascii_digit()) => h,
        Some(_) => return None,
        None => authority,
    };
    (!host.is_empty()).then_some(host)
}

/// Whether `call` may be made by an app whose manifest lists `hosts`; why
/// not, when it may not.
pub fn allowed(call: &Call, hosts: &[String]) -> Result<(), String> {
    if !METHODS.contains(&call.method.as_str()) {
        return Err(format!(
            "`{}` is not a method `ctx.fetch` makes: GET, POST, PUT or DELETE",
            call.method
        ));
    }
    let Some(host) = host_of(&call.url) else {
        return Err(format!(
            "`{}` is not an `https://` address `ctx.fetch` can call",
            call.url
        ));
    };
    if !hosts.iter().any(|h| h == host) {
        return Err(format!(
            "`{host}` is not in this app's `fetch:` list in app.yaml"
        ));
    }
    if let Some(own) = call
        .headers
        .keys()
        .find(|k| OWN_HEADERS.contains(&k.to_ascii_lowercase().as_str()))
    {
        return Err(format!("`ctx.fetch` sets `{own}` itself"));
    }
    Ok(())
}

/// The internet, over HTTPS.
pub struct Https {
    agent: Agent,
}

impl Https {
    pub fn new() -> Https {
        Https {
            agent: AgentBuilder::new()
                .timeout(TIMEOUT)
                .redirects(0)
                .https_only(true)
                .build(),
        }
    }
}

impl Web for Https {
    fn send(&self, call: &Call) -> Result<Reply> {
        let mut req = self.agent.request(&call.method, &call.url);
        for (k, v) in &call.headers {
            req = req.set(k, v);
        }
        let res = match &call.body {
            Some(b) => req.send_string(b),
            None => req.call(),
        };
        let res = match res {
            Ok(r) | Err(Answered::Status(_, r)) => r,
            Err(e) => return Err(anyhow!("{} could not be reached: {e}", call.url)),
        };
        let mut headers = BTreeMap::new();
        for name in res.headers_names() {
            if let Some(v) = res.header(&name) {
                headers.insert(name.to_ascii_lowercase(), v.to_string());
            }
        }
        let status = res.status();
        let mut bytes = Vec::new();
        res.into_reader()
            .take(MAX_ANSWER)
            .read_to_end(&mut bytes)
            .map_err(|e| anyhow!("{}'s answer was cut off: {e}", call.url))?;
        Ok(Reply {
            status,
            headers,
            text: String::from_utf8_lossy(&bytes).into_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(method: &str, url: &str) -> Call {
        Call {
            url: url.into(),
            method: method.into(),
            headers: BTreeMap::new(),
            body: None,
        }
    }

    #[test]
    fn a_listed_host_is_a_domain_name() {
        for yes in ["api.open-meteo.com", "example.com.au", "a1.b-2.io"] {
            assert!(is_host(yes), "{yes}");
        }
        for no in [
            "localhost",
            "10.0.0.1",
            "::1",
            "Example.com",
            "printer.local",
            "x.localhost",
            "-a.com",
            "a..com",
            "https://a.com",
            "a.com/x",
            "a.com:443",
        ] {
            assert!(!is_host(no), "{no}");
        }
    }

    #[test]
    fn the_host_is_read_from_an_https_address() {
        assert_eq!(
            host_of("https://api.open-meteo.com/v1/forecast?x=1"),
            Some("api.open-meteo.com")
        );
        assert_eq!(host_of("https://a.com:8443/x"), Some("a.com"));
        assert_eq!(host_of("https://a.com?q"), Some("a.com"));
        assert_eq!(host_of("https://a.com#f"), Some("a.com"));
        assert_eq!(host_of("http://a.com/"), None);
        assert_eq!(host_of("https://me:pw@a.com/"), None);
        assert_eq!(host_of("https://a.com:x/"), None);
        assert_eq!(host_of("https:///x"), None);
    }

    /// **A call goes only to a listed host, by a method it may make, with
    /// no header the site sets itself.**
    #[test]
    fn a_call_goes_only_where_the_manifest_lists() {
        let hosts = vec!["api.open-meteo.com".to_string()];
        let told = |c: &Call| allowed(c, &hosts).err();
        assert_eq!(
            told(&call("GET", "https://api.open-meteo.com/v1/forecast")),
            None
        );
        assert_eq!(
            told(&call("GET", "https://evil.example/")).as_deref(),
            Some("`evil.example` is not in this app's `fetch:` list in app.yaml")
        );
        assert_eq!(
            told(&call("GET", "https://open-meteo.com/")).as_deref(),
            Some("`open-meteo.com` is not in this app's `fetch:` list in app.yaml"),
            "exactly the host listed"
        );
        assert_eq!(
            told(&call("PATCH", "https://api.open-meteo.com/")).as_deref(),
            Some("`PATCH` is not a method `ctx.fetch` makes: GET, POST, PUT or DELETE")
        );
        assert_eq!(
            told(&call("GET", "http://api.open-meteo.com/")).as_deref(),
            Some("`http://api.open-meteo.com/` is not an `https://` address `ctx.fetch` can call")
        );
        let mut hosted = call("GET", "https://api.open-meteo.com/");
        hosted.headers.insert("Host".into(), "10.0.0.1".into());
        assert_eq!(
            told(&hosted).as_deref(),
            Some("`ctx.fetch` sets `Host` itself")
        );
    }
}
