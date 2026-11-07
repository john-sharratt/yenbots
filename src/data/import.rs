//! Records in (§20.1): JSON as an export writes it, or one collection's CSV
//! from a spreadsheet — each record made new, as the caller, validated
//! against its collection's fields like any write, all in one transaction:
//! the first refused stops the import and nothing is kept.

use std::collections::BTreeMap;

use anyhow::Result;
use rusqlite::Transaction;
use serde_json::{Map, Value};

use super::csv::parse;
use super::docs::{create, Denied};
use crate::accounts::caller::Caller;
use crate::apps::manifest::{FieldType, Manifest, RESERVED};

/// Why an import stopped: the collection, the record's place in it (from
/// 1), and what was refused.
#[derive(Debug)]
pub struct Stopped {
    pub coll: String,
    pub row: usize,
    pub denied: Denied,
}

/// How many records each collection took.
pub type Made = BTreeMap<String, usize>;

/// `fields` without the record's own keys, which an import makes anew.
fn fields_of(record: &Value) -> Value {
    match record {
        Value::Object(o) => Value::Object(
            o.iter()
                .filter(|(k, _)| !RESERVED.contains(&k.as_str()))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        ),
        other => other.clone(),
    }
}

fn make(
    tx: &Transaction<'_>,
    m: &Manifest,
    caller: &Caller,
    coll: &str,
    records: impl Iterator<Item = Result<Value, Denied>>,
    now: i64,
    made: &mut Made,
) -> Result<Result<(), Stopped>> {
    for (i, r) in records.enumerate() {
        let stopped = |denied| Stopped {
            coll: coll.to_string(),
            row: i + 1,
            denied,
        };
        let fields = match r {
            Ok(f) => f,
            Err(d) => return Ok(Err(stopped(d))),
        };
        if let Err(d) = create(tx, m, coll, caller, &fields, now)? {
            return Ok(Err(stopped(d)));
        }
        *made.entry(coll.to_string()).or_default() += 1;
    }
    Ok(Ok(()))
}

/// Import `body`, `{collections: {name: [record]}}` as an export writes it;
/// each record's own keys (`id`, `owner`, …) are dropped and made anew.
pub fn from_json(
    tx: &Transaction<'_>,
    m: &Manifest,
    caller: &Caller,
    body: &Value,
    now: i64,
) -> Result<Result<Made, Stopped>> {
    let mut made = Made::new();
    let Some(collections) = body.get("collections").and_then(Value::as_object) else {
        return Ok(Err(Stopped {
            coll: String::new(),
            row: 0,
            denied: Denied::Bad(
                "an import is `{collections: {name: [record]}}`, as an export writes it".into(),
            ),
        }));
    };
    for (coll, records) in collections {
        let Some(records) = records.as_array() else {
            return Ok(Err(Stopped {
                coll: coll.clone(),
                row: 0,
                denied: Denied::Bad(format!("`{coll}` is a list of records")),
            }));
        };
        let it = records.iter().map(|r| Ok(fields_of(r)));
        if let Err(s) = make(tx, m, caller, coll, it, now, &mut made)? {
            return Ok(Err(s));
        }
    }
    Ok(Ok(made))
}

/// A CSV cell as a value of field type `kind`; `None` for an empty cell,
/// which leaves the field out.
fn value_of(kind: FieldType, cell: &str) -> Result<Option<Value>, String> {
    if cell.is_empty() {
        return Ok(None);
    }
    Ok(Some(match kind {
        FieldType::Number => match cell.trim().parse::<f64>() {
            Ok(n) if n.fract() == 0.0 && n.abs() < 9e15 => Value::from(n as i64),
            Ok(n) => Value::from(n),
            Err(_) => return Err(format!("`{cell}` is not a number")),
        },
        FieldType::Time => match cell.trim().parse::<i64>() {
            Ok(n) => Value::from(n),
            Err(_) => return Err(format!("`{cell}` is not a time in milliseconds")),
        },
        FieldType::Bool => match cell.trim().to_ascii_lowercase().as_str() {
            "true" | "yes" | "1" => Value::Bool(true),
            "false" | "no" | "0" => Value::Bool(false),
            _ => return Err(format!("`{cell}` is not true or false")),
        },
        FieldType::List | FieldType::Object => {
            serde_json::from_str(cell).map_err(|_| format!("`{cell}` is not JSON"))?
        }
        FieldType::Text | FieldType::Date | FieldType::File | FieldType::Ref => {
            Value::String(cell.to_string())
        }
    }))
}

/// Import collection `coll` from CSV `text`: a header naming fields, then a
/// record a line; columns that are the record's own (`id`, `owner`, …) are
/// dropped.
pub fn from_csv(
    tx: &Transaction<'_>,
    m: &Manifest,
    caller: &Caller,
    coll: &str,
    text: &str,
    now: i64,
) -> Result<Result<Made, Stopped>> {
    let bad = |row: usize, why: String| Stopped {
        coll: coll.to_string(),
        row,
        denied: Denied::Bad(why),
    };
    let Some(c) = m.collections.get(coll) else {
        return Ok(Err(Stopped {
            coll: coll.to_string(),
            row: 0,
            denied: Denied::NoCollection(format!("{} has no collection `{coll}`", m.id)),
        }));
    };
    let rows = match parse(text) {
        Ok(r) => r,
        Err(why) => return Ok(Err(bad(0, why))),
    };
    let Some((header, lines)) = rows.split_first() else {
        return Ok(Ok(Made::new()));
    };
    let mut columns = Vec::new();
    for name in header {
        let name = name.trim();
        if RESERVED.contains(&name) {
            columns.push(None);
            continue;
        }
        let Some(f) = c.fields.get(name) else {
            return Ok(Err(bad(0, format!("`{name}` is not a field of {coll}"))));
        };
        columns.push(Some((name.to_string(), f.kind)));
    }
    let records = lines.iter().map(|line| {
        let mut fields = Map::new();
        for (cell, column) in line.iter().zip(&columns) {
            let Some((name, kind)) = column else { continue };
            match value_of(*kind, cell) {
                Ok(Some(v)) => {
                    fields.insert(name.clone(), v);
                }
                Ok(None) => {}
                Err(why) => return Err(Denied::Bad(format!("{name}: {why}"))),
            }
        }
        Ok(Value::Object(fields))
    });
    let mut made = Made::new();
    Ok(make(tx, m, caller, coll, records, now, &mut made)?.map(|()| made))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::data::export::as_csv;
    use crate::store::Store;

    const APP: &str = "id: recipes\nname: Recipes\ncollections:\n  recipes:\n    fields:\n      \
                       title: { type: text, required: true }\n      serves: { type: number }\n      \
                       vegan: { type: bool }\n      tags: { type: list, of: text }\n";

    fn count(store: &Store) -> i64 {
        store
            .conn()
            .query_row("SELECT COUNT(*) FROM docs", [], |r| r.get(0))
            .unwrap()
    }

    /// **A spreadsheet's rows come in typed; what an export wrote comes back
    /// in; a row refused keeps nothing, and says which.**
    #[test]
    fn records_come_in_all_or_nothing() {
        let store = Store::in_memory().unwrap();
        let m = Manifest::parse(APP, "recipes").unwrap();
        let app = Caller::app("recipes", "Recipes");
        let csv = "\u{feff}title,serves,vegan,tags,id\r\nPhở bò,4,no,\"[\"\"soup\"\"]\",x1\r\nGỏi cuốn,,yes,,x2\r\n";
        let mut conn = store.conn();
        let tx = conn.transaction().unwrap();
        let made = from_csv(&tx, &m, &app, "recipes", csv, 5).unwrap().unwrap();
        tx.commit().unwrap();
        assert_eq!(made, Made::from([("recipes".into(), 2)]));
        let out = parse(&as_csv(&conn, &m, "recipes", &app, false).unwrap().unwrap()).unwrap();
        assert_eq!(out[0][2..6], ["serves", "tags", "title", "vegan"]);
        assert_eq!(
            out[1][1..],
            [
                "app:recipes",
                "4",
                "[\"soup\"]",
                "Phở bò",
                "false",
                "5",
                "5"
            ]
        );
        assert_eq!(out[2][2..6], ["", "", "Gỏi cuốn", "true"]);
        drop(conn);
        assert_eq!(count(&store), 2);

        let back = json!({"collections": {"recipes": [{"id": "old", "owner": "9", "title": "Chè", "version": 3}]}});
        let mut conn = store.conn();
        let tx = conn.transaction().unwrap();
        assert_eq!(
            from_json(&tx, &m, &app, &back, 6).unwrap().unwrap(),
            Made::from([("recipes".into(), 1)])
        );
        tx.commit().unwrap();
        drop(conn);
        assert_eq!(count(&store), 3);

        let mut conn = store.conn();
        let tx = conn.transaction().unwrap();
        let stopped = from_csv(&tx, &m, &app, "recipes", "title,serves\nA,1\nB,many\n", 7)
            .unwrap()
            .unwrap_err();
        drop(tx);
        drop(conn);
        assert_eq!((stopped.coll.as_str(), stopped.row), ("recipes", 2));
        assert!(
            matches!(stopped.denied, Denied::Bad(ref w) if w == "serves: `many` is not a number")
        );
        assert_eq!(count(&store), 3, "nothing of a refused import is kept");

        let mut conn = store.conn();
        let tx = conn.transaction().unwrap();
        let missing = from_csv(&tx, &m, &app, "recipes", "serves\n2\n", 8)
            .unwrap()
            .unwrap_err();
        assert!(matches!(missing.denied, Denied::Invalid(_)), "{missing:?}");
        let unknown = from_csv(&tx, &m, &app, "recipes", "colour\nred\n", 8)
            .unwrap()
            .unwrap_err();
        assert!(
            matches!(unknown.denied, Denied::Bad(ref w) if w == "`colour` is not a field of recipes")
        );
    }
}
