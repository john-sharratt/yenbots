//! AI and pictures through Zen (`candle/docs/yenbot_app_host.md` §14): an
//! app's chat with the model, a picture drawn or changed, and a question
//! about its own data — for a page through the API, for a workflow through
//! `ctx.ai` — each only for an app whose manifest says `ai: true`, each
//! call kept in `ai_calls`.
//!
//! | Module | |
//! |---|---|
//! | [`zen`] | Zen through the gateway, as the site's service key |
//! | [`chat`] | a chat with the model, its answer as it is written |
//! | [`schema`] | an answer held to the shape asked |
//! | [`image`] | a picture drawn, or changed |
//! | [`ask`] | a question about a collection, from its records |
//! | [`calls`] | every call kept |

pub mod ask;
pub mod calls;
pub mod chat;
#[cfg(test)]
pub mod fake;
pub mod image;
pub mod schema;
pub mod zen;

use std::time::Instant;

use anyhow::Result;
use serde_json::Value;

use crate::accounts::caller::Caller;
use crate::accounts::now_ms;
use crate::apps::manifest::Manifest;
use crate::files::{Files, Kept};
use crate::jobs::follow::acting;
use crate::store::Store;
use ask::Answered;
use calls::{record, Call};
use chat::{Answer, Ask};
use image::Draw;
use zen::Zen;

/// What an app's call reaches, and who it is for.
pub struct Asker<'a> {
    pub zen: &'a dyn Zen,
    pub store: &'a Store,
    pub files: &'a Files,
    pub m: &'a Manifest,
    pub caller: &'a Caller,
}

/// The refusal an app without `ai: true` gets.
pub const NOT_AI: &str = "this app does not use the AI: add `ai: true` to its app.yaml";

/// A result for the app: what it asked for, or why not, said for it.
pub type Said<T> = std::result::Result<T, String>;

impl Asker<'_> {
    fn keep(
        &self,
        kind: &'static str,
        asked: &str,
        started: Instant,
        tokens: (Option<u64>, Option<u64>),
        error: Option<&str>,
    ) {
        let call = Call {
            app: &self.m.id,
            account: acting(self.caller.id()),
            kind,
            asked,
            ms: started.elapsed().as_millis() as i64,
            tokens_in: tokens.0,
            tokens_out: tokens.1,
            error,
            at_ms: now_ms(),
        };
        if let Err(e) = record(&self.store.conn(), &call) {
            tracing::error!(target: "yenbot::ai", app = %self.m.id, "a call could not be kept: {e:#}");
        }
    }

    /// A chat, its answer handed to `delta` as it is written.
    pub fn chat(&self, ask: &Ask, delta: &mut dyn FnMut(&str)) -> Result<Said<Answer>> {
        if !self.m.ai {
            return Ok(Err(NOT_AI.into()));
        }
        let started = Instant::now();
        let asked = ask
            .messages
            .last()
            .map(|m| m.content.as_str())
            .unwrap_or_default();
        let answer = chat::chat(self.zen, ask, delta);
        match &answer {
            Ok(Ok(a)) => self.keep("chat", asked, started, (a.tokens_in, a.tokens_out), None),
            Ok(Err(why)) => self.keep("chat", asked, started, (None, None), Some(why)),
            Err(e) => self.keep(
                "chat",
                asked,
                started,
                (None, None),
                Some(&format!("{e:#}")),
            ),
        }
        answer
    }

    /// A picture drawn — or one of the app's changed — kept as a file of the
    /// app, its progress handed to `progress`; answers the file and the seed.
    pub fn image(
        &self,
        draw: &Draw,
        progress: &mut dyn FnMut(&Value),
    ) -> Result<Said<(Kept, Option<u64>)>> {
        if !self.m.ai {
            return Ok(Err(NOT_AI.into()));
        }
        let reference = match &draw.from {
            None => None,
            Some(id) => match self.files.get(&self.store.conn(), &self.m.id, id)? {
                Some(k) if k.kind.is_picture() => {
                    Some(std::fs::read(self.files.path(&self.m.id, id, false))?)
                }
                _ => {
                    return Ok(Err(format!(
                        "{} has no picture `{id}` to change",
                        self.m.id
                    )))
                }
            },
        };
        let started = Instant::now();
        let drawn = image::draw(self.zen, draw, reference.as_deref(), progress)?;
        let drawn = match drawn {
            Ok(d) => d,
            Err(why) => {
                self.keep("image", &draw.prompt, started, (None, None), Some(&why));
                return Ok(Err(why));
            }
        };
        let name: String = draw.prompt.chars().take(60).collect();
        let kept = self.files.keep(
            &self.store.conn(),
            &self.m.id,
            self.caller.id(),
            &format!("{name}.png"),
            &drawn.png,
            now_ms(),
        )?;
        self.keep("image", &draw.prompt, started, (None, None), None);
        Ok(Ok((kept, drawn.seed)))
    }

    /// A question about collection `coll`, from its records.
    pub fn ask(&self, coll: &str, question: &str) -> Result<Said<Answered>> {
        if !self.m.ai {
            return Ok(Err(NOT_AI.into()));
        }
        let started = Instant::now();
        let answered = ask::ask(self.zen, self.store, self.m, coll, self.caller, question)?;
        match &answered {
            Ok(a) => self.keep("ask", question, started, a.tokens, None),
            Err(why) => self.keep("ask", question, started, (None, None), Some(why)),
        }
        Ok(answered)
    }
}
