//! A Zen for tests: each call answers the next of the lines it was given,
//! and what it was sent is kept.

use std::sync::Mutex;

use anyhow::{anyhow, Result};
use serde_json::Value;

use super::zen::Zen;

#[derive(Default)]
pub struct FakeZen {
    chats: Mutex<Vec<Vec<String>>>,
    images: Mutex<Vec<Vec<String>>>,
    sent: Mutex<Vec<Value>>,
}

impl FakeZen {
    /// A Zen whose chats answer `answers`, in turn.
    pub fn chats(answers: Vec<Vec<String>>) -> FakeZen {
        FakeZen {
            chats: Mutex::new(answers),
            ..FakeZen::default()
        }
    }

    /// A Zen whose pictures answer `answers`, in turn.
    pub fn images(answers: Vec<Vec<String>>) -> FakeZen {
        FakeZen {
            images: Mutex::new(answers),
            ..FakeZen::default()
        }
    }

    /// What it was sent, in turn.
    pub fn sent(&self) -> Vec<Value> {
        self.sent.lock().unwrap().clone()
    }

    fn answer(
        &self,
        from: &Mutex<Vec<Vec<String>>>,
        body: &Value,
        line: &mut dyn FnMut(&str),
    ) -> Result<()> {
        self.sent.lock().unwrap().push(body.clone());
        let mut left = from.lock().unwrap();
        if left.is_empty() {
            return Err(anyhow!("the fake Zen has no answer left"));
        }
        for l in left.remove(0) {
            line(&l);
        }
        Ok(())
    }
}

impl Zen for FakeZen {
    fn chat(&self, body: &Value, line: &mut dyn FnMut(&str)) -> Result<()> {
        self.answer(&self.chats, body, line)
    }

    fn image(&self, body: &Value, line: &mut dyn FnMut(&str)) -> Result<()> {
        self.answer(&self.images, body, line)
    }
}
