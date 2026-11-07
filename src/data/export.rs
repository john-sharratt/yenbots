//! An app's records out (§20.1): every collection as JSON, or one as CSV
//! for a spreadsheet — the records the caller may read, or, for an admin,
//! every account's.

use anyhow::Result;
use rusqlite::Connection;
use serde_json::{json, Map, Value};

use super::csv::{write_row, BOM};
use super::docs::{query, Answer, Query, MAX_PAGE};
use crate::accounts::caller::Caller;
use crate::apps::manifest::Manifest;

/// The record's own columns a CSV has before its fields, and after.
const LEADING: [&str; 2] = ["id", "owner"];
const TRAILING: [&str; 2] = ["created_ms", "updated_ms"];

/// Every record of `coll` the caller may read — every account's when
/// `all` — in creation order.
pub fn records(
    conn: &Connection,
    m: &Manifest,
    coll: &str,
    caller: &Caller,
    all: bool,
) -> Result<Answer<Vec<Value>>> {
    let mut out = Vec::new();
    let mut after = None;
    loop {
        let q = Query {
            filter: None,
            order: None,
            limit: Some(MAX_PAGE),
            after: after.take(),
        };
        let page = match query(conn, m, coll, caller, &q, all)? {
            Ok(p) => p,
            Err(d) => return Ok(Err(d)),
        };
        out.extend(page.docs.iter().map(|d| d.record()));
        match page.after {
            Some(next) => after = Some(next),
            None => return Ok(Ok(out)),
        }
    }
}

/// The app's records as JSON: `{app, collections: {name: [record]}}`.
pub fn as_json(
    conn: &Connection,
    m: &Manifest,
    caller: &Caller,
    all: bool,
) -> Result<Answer<Value>> {
    let mut collections = Map::new();
    for coll in m.collections.keys() {
        match records(conn, m, coll, caller, all)? {
            Ok(r) => collections.insert(coll.clone(), Value::from(r)),
            Err(d) => return Ok(Err(d)),
        };
    }
    Ok(Ok(json!({"app": m.id, "collections": collections})))
}

/// A cell as a spreadsheet shows it: text as it is, a list or an object as
/// JSON, nothing for a field left out.
fn cell(v: Option<&Value>) -> String {
    match v {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

/// Collection `coll` as CSV: its id and owner, its fields in name order,
/// and when each was made and changed.
pub fn as_csv(
    conn: &Connection,
    m: &Manifest,
    coll: &str,
    caller: &Caller,
    all: bool,
) -> Result<Answer<String>> {
    let rows = match records(conn, m, coll, caller, all)? {
        Ok(r) => r,
        Err(d) => return Ok(Err(d)),
    };
    let fields: Vec<&String> = m.collections[coll].fields.keys().collect();
    let header: Vec<String> = LEADING
        .iter()
        .map(|s| s.to_string())
        .chain(fields.iter().map(|f| f.to_string()))
        .chain(TRAILING.iter().map(|s| s.to_string()))
        .collect();
    let mut out = String::from(BOM);
    write_row(&mut out, &header);
    for r in &rows {
        let cells: Vec<String> = header.iter().map(|h| cell(r.get(h.as_str()))).collect();
        write_row(&mut out, &cells);
    }
    Ok(Ok(out))
}

#[cfg(test)]
mod tests {
    use rusqlite::params;

    use super::*;
    use crate::data::csv::parse;

    const APP: &str = "id: recipes\nname: Recipes\ncollections:\n  recipes:\n    fields:\n      \
                       title: { type: text }\n      serves: { type: number }\n      tags: { type: list, of: text }\n";

    /// **A collection comes out whole, every page of it, as JSON and as CSV
    /// a spreadsheet reads with its accents.**
    #[test]
    fn an_apps_records_come_out() {
        let store = crate::store::Store::in_memory().unwrap();
        let m = Manifest::parse(APP, "recipes").unwrap();
        let app = Caller::app("recipes", "Recipes");
        let conn = store.conn();
        for i in 0..(MAX_PAGE + 5) {
            conn.execute(
                "INSERT INTO docs (app, coll, id, owner, data, version, created_ms, updated_ms) \
                 VALUES ('recipes', 'recipes', ?1, '1', ?2, 1, ?3, ?3)",
                params![
                    format!("r{i:04}"),
                    json!({"title": format!("Phở {i}"), "serves": 2, "tags": ["soup", "a,b"]})
                        .to_string(),
                    i as i64
                ],
            )
            .unwrap();
        }
        let all = as_json(&conn, &m, &app, false).unwrap().unwrap();
        assert_eq!(
            all["collections"]["recipes"].as_array().unwrap().len(),
            MAX_PAGE + 5
        );

        let csv = as_csv(&conn, &m, "recipes", &app, false).unwrap().unwrap();
        assert!(csv.starts_with('\u{feff}'));
        let rows = parse(&csv).unwrap();
        assert_eq!(
            rows[0],
            [
                "id",
                "owner",
                "serves",
                "tags",
                "title",
                "created_ms",
                "updated_ms"
            ]
        );
        assert_eq!(
            rows[1],
            ["r0000", "1", "2", "[\"soup\",\"a,b\"]", "Phở 0", "0", "0"]
        );
        assert_eq!(rows.len(), MAX_PAGE + 6);
    }
}
