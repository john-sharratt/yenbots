//! A web for tests: answers each call with the replies it was given, in
//! turn, and keeps every call it was asked to make.

use std::collections::VecDeque;
use std::sync::Mutex;

use anyhow::{anyhow, Result};

use super::outbound::{Call, Reply, Web};

#[derive(Default)]
pub struct FakeWeb {
    replies: Mutex<VecDeque<Reply>>,
    sent: Mutex<Vec<Call>>,
}

impl FakeWeb {
    pub fn replying(replies: Vec<Reply>) -> FakeWeb {
        FakeWeb {
            replies: Mutex::new(replies.into()),
            sent: Mutex::default(),
        }
    }

    /// The calls made, in order.
    pub fn sent(&self) -> Vec<Call> {
        self.sent.lock().unwrap().clone()
    }
}

impl Web for FakeWeb {
    fn send(&self, call: &Call) -> Result<Reply> {
        self.sent.lock().unwrap().push(call.clone());
        self.replies
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| anyhow!("{} could not be reached: no reply", call.url))
    }
}
