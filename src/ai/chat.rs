//! A chat with the model (§14): the app's own messages and system prompt,
//! sent to zend as `model: "passthrough"` — none of Zen's coding projection
//! — its answer handed on a piece at a time as it is written.
//!
//! An answer asked to fit a JSON schema is asked for as JSON alone, read
//! whole, and held to the schema's shape ([`fits`]); one that does not fit
//! is asked for once more, told why.

use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};

use super::schema::fits;
use super::zen::Zen;

/// The longest answer an app may ask for, in tokens.
pub const MAX_TOKENS: u32 = 4_000;

/// One message of the conversation, as the app sends it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Said {
    pub role: String,
    pub content: String,
}

/// What an app asks.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Ask {
    /// How the model should answer: the app's own system prompt.
    #[serde(default)]
    pub system: Option<String>,
    pub messages: Vec<Said>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    /// The shape the answer must have, as a JSON Schema.
    #[serde(default)]
    pub schema: Option<Value>,
}

/// The model's answer.
#[derive(Debug, Clone, PartialEq)]
pub struct Answer {
    pub content: String,
    /// The answer read as JSON, when a schema was asked for.
    pub json: Option<Value>,
    pub tokens_in: Option<u64>,
    pub tokens_out: Option<u64>,
}

impl Ask {
    /// Why the ask cannot be sent, if it cannot.
    pub fn checked(&self) -> std::result::Result<(), String> {
        let Some(last) = self.messages.last() else {
            return Err("a chat has at least one message".into());
        };
        if last.role != "user" {
            return Err("the last message is the person's, `role: \"user\"`".into());
        }
        if let Some(bad) = self
            .messages
            .iter()
            .find(|m| m.role != "user" && m.role != "assistant")
        {
            return Err(format!(
                "`{}` is not a message's role: `user` or `assistant`; the app's own instructions are `system`",
                bad.role
            ));
        }
        Ok(())
    }

    /// The system prompt, with the schema's instructions when one is asked.
    fn system_prompt(&self) -> Option<String> {
        let schema = self.schema.as_ref().map(|s| {
            format!(
                "Answer with JSON alone — no words, no code fence around it — that matches this JSON Schema:\n{s}"
            )
        });
        match (&self.system, schema) {
            (Some(s), Some(j)) => Some(format!("{s}\n\n{j}")),
            (Some(s), None) => Some(s.clone()),
            (None, j) => j,
        }
    }

    /// The request zend is sent.
    pub fn body(&self, extra: &[Said]) -> Value {
        let mut messages = Vec::new();
        if let Some(s) = self.system_prompt() {
            messages.push(json!({"role": "system", "content": s}));
        }
        for m in self.messages.iter().chain(extra) {
            messages.push(json!({"role": m.role, "content": m.content}));
        }
        json!({
            "model": "passthrough",
            "messages": messages,
            "stream": true,
            "stream_options": {"include_usage": true},
            "max_tokens": self.max_tokens.unwrap_or(1_000).min(MAX_TOKENS),
        })
    }
}

/// What a stream of server-sent events said.
#[derive(Debug, Default)]
struct Heard {
    content: String,
    tokens_in: Option<u64>,
    tokens_out: Option<u64>,
    error: Option<String>,
}

/// Read zend's stream, a line at a time, handing each piece of the answer
/// to `delta`.
fn hear(zen: &dyn Zen, body: &Value, delta: &mut dyn FnMut(&str)) -> Result<Heard> {
    let mut heard = Heard::default();
    let mut event = String::new();
    zen.chat(body, &mut |line: &str| {
        if line.is_empty() {
            event.clear();
            return;
        }
        if let Some(name) = line.strip_prefix("event:") {
            event = name.trim().to_string();
            return;
        }
        let Some(data) = line.strip_prefix("data:").map(str::trim) else {
            return;
        };
        if data == "[DONE]" {
            return;
        }
        let Ok(v) = serde_json::from_str::<Value>(data) else {
            return;
        };
        match event.as_str() {
            "turn_error" => {
                heard.error = Some(
                    v["message"]
                        .as_str()
                        .unwrap_or("the model failed")
                        .to_string(),
                );
            }
            // Zen's own events — its status, its thinking — are not the answer.
            "" => {
                if let Some(piece) = v["choices"][0]["delta"]["content"].as_str() {
                    heard.content.push_str(piece);
                    delta(piece);
                }
                if let Some(u) = v.get("usage").filter(|u| u.is_object()) {
                    heard.tokens_in = u["prompt_tokens"].as_u64();
                    heard.tokens_out = u["completion_tokens"].as_u64();
                }
            }
            _ => {}
        }
    })?;
    Ok(heard)
}

/// The JSON an answer holds: itself, or inside a code fence the model put
/// round it anyway.
fn json_of(content: &str) -> std::result::Result<Value, String> {
    let t = content.trim();
    let t = t
        .strip_prefix("```json")
        .or_else(|| t.strip_prefix("```"))
        .and_then(|r| r.strip_suffix("```"))
        .unwrap_or(t)
        .trim();
    serde_json::from_str(t).map_err(|e| format!("it is not JSON: {e}"))
}

/// Ask the model `ask`, handing each piece of the answer to `delta` as it
/// is written — none, for an answer held to a schema, which is read whole.
/// `Err` is what went wrong, said for the app.
pub fn chat(
    zen: &dyn Zen,
    ask: &Ask,
    delta: &mut dyn FnMut(&str),
) -> Result<std::result::Result<Answer, String>> {
    if let Err(why) = ask.checked() {
        return Ok(Err(why));
    }
    let Some(schema) = &ask.schema else {
        let heard = hear(zen, &ask.body(&[]), delta)?;
        if let Some(e) = heard.error {
            return Ok(Err(e));
        }
        return Ok(Ok(Answer {
            content: heard.content,
            json: None,
            tokens_in: heard.tokens_in,
            tokens_out: heard.tokens_out,
        }));
    };
    let mut retold = Vec::new();
    let mut spent = (0u64, 0u64);
    for _ in 0..2 {
        let heard = hear(zen, &ask.body(&retold), &mut |_| {})?;
        if let Some(e) = heard.error {
            return Ok(Err(e));
        }
        spent.0 += heard.tokens_in.unwrap_or(0);
        spent.1 += heard.tokens_out.unwrap_or(0);
        let read = json_of(&heard.content).and_then(|v| fits(&v, schema, "the answer").map(|()| v));
        match read {
            Ok(v) => {
                return Ok(Ok(Answer {
                    content: heard.content,
                    json: Some(v),
                    tokens_in: Some(spent.0),
                    tokens_out: Some(spent.1),
                }))
            }
            Err(why) => {
                retold.push(Said {
                    role: "assistant".into(),
                    content: heard.content,
                });
                retold.push(Said {
                    role: "user".into(),
                    content: format!(
                        "That does not match the schema: {why}. Answer again, with the JSON alone."
                    ),
                });
            }
        }
    }
    Ok(Err(
        "the model's answer did not match the schema, twice".into()
    ))
}

#[cfg(test)]
#[path = "chat_tests.rs"]
mod tests;
