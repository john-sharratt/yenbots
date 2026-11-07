//! Asking about an app's own data (§14): "which of my recipes use chicken?"
//! is search first, then the model. The collection's searchable fields are
//! searched for each word of the question, as the caller may see them; the
//! best matches go to the model with their ids, and it answers with the ids
//! it drew on, so the app can link to them.

use std::collections::BTreeMap;

use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;
use serde_json::{json, Value};

use super::chat::{chat, Ask, Said};
use super::zen::Zen;
use crate::accounts::caller::Caller;
use crate::apps::manifest::Manifest;
use crate::data::docs::{self, Doc};
use crate::store::Store;

/// The most records put before the model.
pub const RECORDS: usize = 12;

/// The longest a record is as the model reads it, in characters.
const RECORD_CHARS: usize = 800;

/// Words too common to search for.
const COMMON: &[&str] = &[
    "the", "and", "for", "with", "that", "this", "what", "which", "who", "how", "are", "was",
    "were", "have", "has", "had", "use", "uses", "used", "any", "all", "our", "your", "my", "mine",
    "from", "into", "about", "does", "did", "can", "there", "their", "them", "they", "when",
    "where", "why", "some", "much", "many",
];

/// The model's answer and the records it drew on.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Answered {
    pub answer: String,
    /// The ids of the records it drew on, each one it was shown.
    pub ids: Vec<String>,
    /// How many records it was shown.
    pub read: usize,
    #[serde(skip)]
    pub tokens: (Option<u64>, Option<u64>),
}

/// The words of `question` worth searching for.
pub fn words_of(question: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for w in question.split(|c: char| !c.is_alphanumeric()) {
        let w = w.to_lowercase();
        if w.chars().count() >= 3 && !COMMON.contains(&w.as_str()) && !out.contains(&w) {
            out.push(w);
        }
    }
    out
}

/// The records of `coll` the caller may see that best match `question`:
/// those matching most of its words first.
fn matches(
    conn: &Connection,
    m: &Manifest,
    coll: &str,
    caller: &Caller,
    question: &str,
) -> Result<std::result::Result<Vec<Doc>, String>> {
    let mut hits: BTreeMap<String, (usize, Doc)> = BTreeMap::new();
    for w in words_of(question) {
        match docs::search(conn, m, coll, caller, &w, RECORDS)? {
            Ok(found) => {
                for d in found {
                    hits.entry(d.id.clone()).or_insert((0, d)).0 += 1;
                }
            }
            Err(d) => return Ok(Err(format!("{d:?}"))),
        }
    }
    let mut ranked: Vec<(usize, Doc)> = hits.into_values().collect();
    ranked.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.updated_ms.cmp(&a.1.updated_ms)));
    Ok(Ok(ranked
        .into_iter()
        .take(RECORDS)
        .map(|(_, d)| d)
        .collect()))
}

/// A record as the model reads it.
fn shown(d: &Doc) -> String {
    let text = d.data.to_string();
    let cut: String = text.chars().take(RECORD_CHARS).collect();
    format!("[{}] {cut}", d.id)
}

/// Answer `question` from `coll` of app `m`, as `caller`. The store is
/// taken for the search alone: the model's answer takes seconds.
pub fn ask(
    zen: &dyn Zen,
    store: &Store,
    m: &Manifest,
    coll: &str,
    caller: &Caller,
    question: &str,
) -> Result<std::result::Result<Answered, String>> {
    let Some(c) = m.collections.get(coll) else {
        return Ok(Err(format!("{} has no collection `{coll}`", m.id)));
    };
    if c.search.is_empty() {
        return Ok(Err(format!(
            "`{coll}` names no fields to search, so nothing can be asked of it"
        )));
    }
    if question.trim().is_empty() {
        return Ok(Err("a question has words".into()));
    }
    let searched = matches(&store.conn(), m, coll, caller, question)?;
    let found = match searched {
        Ok(f) => f,
        Err(why) => return Ok(Err(why)),
    };
    let listed: Vec<String> = found.iter().map(shown).collect();
    let records = if listed.is_empty() {
        "(none matched)".to_string()
    } else {
        listed.join("\n")
    };
    let request = Ask {
        system: Some(format!(
            "You answer a question about the person's own {coll} in the app {}, from these records alone — \
             each is its id in brackets, then its fields. If they do not answer it, say so plainly. \
             Give the ids of the records you drew on.\n\nRecords:\n{records}",
            m.name
        )),
        messages: vec![Said {
            role: "user".into(),
            content: question.trim().to_string(),
        }],
        max_tokens: Some(800),
        schema: Some(json!({
            "type": "object",
            "required": ["answer", "ids"],
            "properties": {"answer": {"type": "string"}, "ids": {"type": "array", "items": {"type": "string"}}}
        })),
    };
    let answer = match chat(zen, &request, &mut |_| {})? {
        Ok(a) => a,
        Err(why) => return Ok(Err(why)),
    };
    let v = answer.json.unwrap_or(Value::Null);
    let shown_ids: Vec<&str> = found.iter().map(|d| d.id.as_str()).collect();
    let ids = v["ids"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .filter(|id| shown_ids.contains(id))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    Ok(Ok(Answered {
        answer: v["answer"].as_str().unwrap_or_default().to_string(),
        ids,
        read: found.len(),
        tokens: (answer.tokens_in, answer.tokens_out),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounts::{Account, Status};
    use crate::ai::fake::FakeZen;
    use crate::data::docs::create;

    fn yen() -> Caller {
        Caller {
            account: Account {
                id: "1".into(),
                email: "yen@example.com".into(),
                name: "Yen".into(),
                picture: None,
                status: Status::Active,
                admin: false,
                created_ms: 0,
                seen_ms: 0,
            },
            groups: Vec::new(),
            token: None,
        }
    }

    #[test]
    fn a_question_keeps_the_words_worth_searching() {
        assert_eq!(
            words_of("Which of my recipes use chicken?"),
            ["recipes", "chicken"]
        );
        assert_eq!(words_of("Phở, phở and bún"), ["phở", "bún"]);
    }

    /// **The matching records go to the model with their ids, and only ids
    /// it was shown come back.**
    #[test]
    fn a_question_is_answered_from_the_records() {
        let store = Store::in_memory().unwrap();
        let m = Manifest::parse(
            "id: recipes\nname: Recipes\ncollections:\n  recipes:\n    scope: private\n    fields:\n      title: { type: text }\n    search: [title]\n",
            "recipes",
        )
        .unwrap();
        let mut conn = store.conn();
        let tx = conn.transaction().unwrap();
        let chicken = create(
            &tx,
            &m,
            "recipes",
            &yen(),
            &json!({"title": "Chicken phở"}),
            1,
        )
        .unwrap()
        .unwrap();
        create(&tx, &m, "recipes", &yen(), &json!({"title": "Bún chả"}), 2)
            .unwrap()
            .unwrap();
        tx.commit().unwrap();
        drop(conn);
        let said =
            json!({"answer": "One: chicken phở.", "ids": [chicken.id, "made-up"]}).to_string();
        let zen = FakeZen::chats(vec![vec![format!(
            "data: {}",
            json!({"choices": [{"delta": {"content": said}}]})
        )]]);
        let a = ask(
            &zen,
            &store,
            &m,
            "recipes",
            &yen(),
            "Which recipes use chicken?",
        )
        .unwrap()
        .unwrap();
        assert_eq!(a.answer, "One: chicken phở.");
        assert_eq!(a.ids, std::slice::from_ref(&chicken.id));
        assert_eq!(a.read, 1);
        let system = zen.sent()[0]["messages"][0]["content"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(
            system.contains(&format!("[{}] {{\"title\":\"Chicken phở\"}}", chicken.id)),
            "{system}"
        );
        assert!(!system.contains("Bún"));

        assert_eq!(
            ask(&zen, &store, &m, "nope", &yen(), "x").unwrap(),
            Err("recipes has no collection `nope`".into())
        );
    }
}
