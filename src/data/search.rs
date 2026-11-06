//! Full-text search (§7.6, §8.7): each record's `search:` fields kept in an
//! FTS5 index, updated in the write's own transaction. The tokenizer folds
//! diacritics (`remove_diacritics 2`), so "pho" finds "phở"; `đ`, a letter of
//! its own rather than a marked `d`, is folded here, on the way in and the way
//! out alike.

use anyhow::Result;
use rusqlite::{params, Transaction};
use serde_json::Value;

/// `text` with the letters the tokenizer keeps apart folded together.
pub fn fold(text: &str) -> String {
    text.replace(['đ', 'Đ'], "d")
}

/// What a record's searchable fields say, as one body.
pub fn body_of(fields: &[String], data: &Value) -> String {
    let mut out = Vec::new();
    for f in fields {
        match data.get(f) {
            Some(Value::String(s)) => out.push(s.clone()),
            Some(Value::Array(items)) => {
                out.extend(items.iter().filter_map(|i| i.as_str().map(str::to_string)))
            }
            Some(v @ Value::Number(_)) => out.push(v.to_string()),
            _ => {}
        }
    }
    fold(&out.join("\n"))
}

pub fn index(tx: &Transaction<'_>, app: &str, coll: &str, id: &str, body: &str) -> Result<()> {
    if !body.is_empty() {
        tx.execute(
            "INSERT INTO docs_fts (app, coll, id, body) VALUES (?1, ?2, ?3, ?4)",
            params![app, coll, id, body],
        )?;
    }
    Ok(())
}

pub fn unindex(tx: &Transaction<'_>, app: &str, coll: &str, id: &str) -> Result<()> {
    tx.execute(
        "DELETE FROM docs_fts WHERE app = ?1 AND coll = ?2 AND id = ?3",
        params![app, coll, id],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn the_body_is_the_searchable_fields_folded() {
        let data = json!({"title": "Bánh đa cua", "tags": ["soup", 3], "serves": 4, "notes": "x"});
        assert_eq!(
            body_of(&["title".into(), "tags".into(), "serves".into()], &data),
            "Bánh da cua\nsoup\n4"
        );
    }
}
