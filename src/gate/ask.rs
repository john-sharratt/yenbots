//! What the gate is asked on `gate/yenbot` and what it answers on
//! `gate/yenbot/result/<id>` (`candle/docs/mcp_file_tools.md` §5.4).

use serde::Deserialize;
use serde_json::{json, Value};

/// A push Zen is about to make: the candidate it pushed, and the branch it
/// would move.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Ask {
    pub id: String,
    /// The candidate's ref on origin, `refs/heads/zen/candidate/<id>`;
    /// `None` for a deletion.
    #[serde(rename = "ref")]
    pub reference: Option<String>,
    pub commit: Option<String>,
    /// What the branch held, `None` for a new branch.
    pub base: Option<String>,
    pub branch: String,
    #[serde(default)]
    pub delete: bool,
}

impl Ask {
    pub fn of(payload: &Value) -> Option<Ask> {
        serde_json::from_value(payload.clone()).ok()
    }

    /// Whether `id` is one a ref may be named with: letters, digits, `-`.
    pub fn id_is_plain(&self) -> bool {
        !self.id.is_empty()
            && self.id.len() <= 64
            && self
                .id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-')
    }

    /// The topic the answer goes on.
    pub fn reply_topic(&self) -> String {
        format!("gate/yenbot/result/{}", self.id)
    }
}

/// The gate's answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    Ok,
    Refused(String),
}

impl Answer {
    pub fn payload(&self) -> Value {
        match self {
            Answer::Ok => json!({"ok": true}),
            Answer::Refused(reason) => json!({"ok": false, "reason": reason}),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_reads_as_the_bus_carries_it() {
        let ask = Ask::of(
            &json!({"id": "3f2a", "ref": "refs/heads/zen/candidate/3f2a", "commit": "4f1c",
                                  "base": "a3f9", "branch": "main", "paths": ["apps/r/a.js"]}),
        )
        .unwrap();
        assert_eq!(
            ask,
            Ask {
                id: "3f2a".into(),
                reference: Some("refs/heads/zen/candidate/3f2a".into()),
                commit: Some("4f1c".into()),
                base: Some("a3f9".into()),
                branch: "main".into(),
                delete: false,
            }
        );
        assert_eq!(ask.reply_topic(), "gate/yenbot/result/3f2a");
        let gone = Ask::of(&json!({"id": "9", "ref": null, "commit": null, "base": "a3f9", "branch": "x", "delete": true})).unwrap();
        assert!(gone.delete);
        assert_eq!(Ask::of(&json!({"id": 3})), None);
    }

    #[test]
    fn only_a_plain_id_names_a_ref() {
        let with = |id: &str| {
            Ask::of(&json!({"id": id, "ref": null, "commit": null, "base": null, "branch": "main"}))
                .unwrap()
        };
        assert!(with("3f2a-9").id_is_plain());
        assert!(!with("../x").id_is_plain());
        assert!(!with("").id_is_plain());
    }

    #[test]
    fn an_answer_is_ok_or_a_reason() {
        assert_eq!(Answer::Ok.payload(), json!({"ok": true}));
        assert_eq!(
            Answer::Refused("why".into()).payload(),
            json!({"ok": false, "reason": "why"})
        );
    }
}
