//! How the site reaches Zen (§14): through the gateway on this machine, as
//! itself — the bus's service key, `Authorization: Bearer` — to zend's
//! OpenAI-compatible chat (`Host: code.tokera.com`) and npcd's image engine
//! (`Host: bot.tokera.com`), so the cluster routes each to a ready member.
//! Nothing outside the site calls Zen, and no browser holds the key.
//!
//! Both answers stream, a line at a time: the chat as server-sent events,
//! the image engine as one JSON object a line. [`Zen`] hands each line to
//! the caller as it arrives.

use std::io::{BufRead, BufReader};
use std::time::Duration;

use anyhow::{anyhow, Result};
use serde_json::Value;
use ureq::{Agent, AgentBuilder, Error as Refused};

/// The host zend is reached by, on the gateway.
pub const TEXT_HOST: &str = "code.tokera.com";

/// The host the image engine is reached by.
pub const IMAGE_HOST: &str = "bot.tokera.com";

/// What the site asks Zen, a line of the answer at a time.
pub trait Zen: Send + Sync {
    /// Post `body` to zend's `/v1/chat/completions`, handing each line of
    /// the answer to `line`.
    fn chat(&self, body: &Value, line: &mut dyn FnMut(&str)) -> Result<()>;

    /// Post `body` to the image engine's `/v1/image/generate`, handing each
    /// line of the answer to `line`.
    fn image(&self, body: &Value, line: &mut dyn FnMut(&str)) -> Result<()>;
}

/// Zen through the gateway.
pub struct Gateway {
    base: String,
    key: String,
    agent: Agent,
}

impl Gateway {
    /// The gateway at `base` (`http://127.0.0.1`), called with `key`.
    pub fn new(base: &str, key: &str) -> Gateway {
        Gateway {
            base: base.trim_end_matches('/').to_string(),
            key: key.to_string(),
            // A picture takes minutes; an answer stops between its words for
            // no longer than this.
            agent: AgentBuilder::new()
                .timeout_connect(Duration::from_secs(10))
                .timeout_read(Duration::from_secs(300))
                .build(),
        }
    }

    fn post(&self, host: &str, path: &str, body: &Value, line: &mut dyn FnMut(&str)) -> Result<()> {
        let res = self
            .agent
            .post(&format!("{}{path}", self.base))
            .set("Host", host)
            .set("Authorization", &format!("Bearer {}", self.key))
            .set("Content-Type", "application/json")
            .send_string(&body.to_string());
        let res = match res {
            Ok(r) => r,
            Err(Refused::Status(code, r)) => {
                let said = r.into_string().unwrap_or_default();
                return Err(anyhow!(
                    "Zen answered {code}: {}",
                    said.chars().take(400).collect::<String>()
                ));
            }
            Err(e) => return Err(anyhow!("Zen could not be reached: {e}")),
        };
        for l in BufReader::new(res.into_reader()).lines() {
            let l = l.map_err(|e| anyhow!("Zen's answer was cut off: {e}"))?;
            line(&l);
        }
        Ok(())
    }
}

impl Zen for Gateway {
    fn chat(&self, body: &Value, line: &mut dyn FnMut(&str)) -> Result<()> {
        self.post(TEXT_HOST, "/v1/chat/completions", body, line)
    }

    fn image(&self, body: &Value, line: &mut dyn FnMut(&str)) -> Result<()> {
        self.post(IMAGE_HOST, "/v1/image/generate", body, line)
    }
}
