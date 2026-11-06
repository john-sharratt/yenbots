//! A write checked against its collection's fields (§7.5): every field it
//! names must be declared and of its type, within its bounds; a new record
//! must carry every required field and takes each default it leaves out. A
//! refusal names the field and the reason, which the framework's forms show
//! beside the field. Every string written is stored in NFC ([`nfc`]).

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::{Map, Value};

use super::nfc::nfc;
use crate::apps::manifest::{Field, FieldType};

/// Why a write was refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Refused {
    pub field: String,
    pub reason: String,
}

fn refused(field: &str, reason: impl Into<String>) -> Refused {
    Refused {
        field: field.into(),
        reason: reason.into(),
    }
}

/// Whether `value` is a `YYYY-MM-DD` date.
fn is_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && s.chars()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
}

fn check_kind(name: &str, kind: FieldType, value: &Value) -> Result<(), Refused> {
    let ok = match kind {
        FieldType::Text | FieldType::File | FieldType::Ref => value.is_string(),
        FieldType::Number => value.is_number(),
        FieldType::Bool => value.is_boolean(),
        FieldType::Time => value.as_i64().is_some(),
        FieldType::Date => value.as_str().is_some_and(is_date),
        FieldType::List => value.is_array(),
        FieldType::Object => value.is_object(),
    };
    if ok {
        Ok(())
    } else {
        Err(refused(name, format!("must be {}", describe(kind))))
    }
}

fn describe(kind: FieldType) -> &'static str {
    match kind {
        FieldType::Text => "text",
        FieldType::Number => "a number",
        FieldType::Bool => "true or false",
        FieldType::Time => "a time, in milliseconds",
        FieldType::Date => "a date, YYYY-MM-DD",
        FieldType::File => "a file's id",
        FieldType::List => "a list",
        FieldType::Object => "an object",
        FieldType::Ref => "a record's id",
    }
}

fn check_field(name: &str, f: &Field, value: &Value) -> Result<(), Refused> {
    if value.is_null() {
        return if f.required {
            Err(refused(name, "is required"))
        } else {
            Ok(())
        };
    }
    check_kind(name, f.kind, value)?;
    let size = match value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => Some(s.chars().count() as f64),
        Value::Array(a) => Some(a.len() as f64),
        _ => None,
    };
    let unit = match value {
        Value::Number(_) => "",
        Value::Array(_) => " items",
        _ => " characters",
    };
    if let (Some(size), Some(min)) = (size, f.min) {
        if size < min {
            return Err(refused(name, format!("must be at least {min}{unit}")));
        }
    }
    if let (Some(size), Some(max)) = (size, f.max) {
        if size > max {
            return Err(refused(name, format!("must be at most {max}{unit}")));
        }
    }
    if let (Value::Array(items), Some(of)) = (value, f.of) {
        for (i, item) in items.iter().enumerate() {
            check_kind(&format!("{name}[{i}]"), of, item)?;
        }
    }
    Ok(())
}

/// `data`, a new record, checked and given its defaults.
pub fn create(fields: &BTreeMap<String, Field>, data: &Value) -> Result<Value, Refused> {
    let Value::Object(given) = data else {
        return Err(refused("", "a record is an object of fields"));
    };
    let mut out = Map::new();
    for name in given.keys() {
        if !fields.contains_key(name) {
            return Err(refused(name, "is not a field of this collection"));
        }
    }
    for (name, f) in fields {
        let value = given
            .get(name)
            .cloned()
            .map(nfc)
            .or_else(|| f.default.clone())
            .unwrap_or(Value::Null);
        check_field(name, f, &value)?;
        if !value.is_null() {
            out.insert(name.clone(), value);
        }
    }
    Ok(Value::Object(out))
}

/// `held` with `patch` merged in — a field set to null is removed — checked.
pub fn update(
    fields: &BTreeMap<String, Field>,
    held: &Value,
    patch: &Value,
) -> Result<Value, Refused> {
    let Value::Object(patch) = patch else {
        return Err(refused("", "an update is an object of fields"));
    };
    let mut merged = held.as_object().cloned().unwrap_or_default();
    for (name, value) in patch {
        let Some(f) = fields.get(name) else {
            return Err(refused(name, "is not a field of this collection"));
        };
        let value = nfc(value.clone());
        check_field(name, f, &value)?;
        if value.is_null() {
            merged.remove(name);
        } else {
            merged.insert(name.clone(), value);
        }
    }
    for (name, f) in fields {
        if f.required && !merged.contains_key(name) {
            return Err(refused(name, "is required"));
        }
    }
    Ok(Value::Object(merged))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn fields() -> BTreeMap<String, Field> {
        serde_yaml::from_str(
            "title:  { type: text, required: true, max: 10 }\n\
             serves: { type: number, min: 1, default: 2 }\n\
             tags:   { type: list, of: text, max: 3 }\n\
             cooked: { type: date }\n",
        )
        .unwrap()
    }

    #[test]
    fn a_new_record_takes_its_defaults() {
        assert_eq!(
            create(&fields(), &json!({"title": "Phở"})),
            Ok(json!({"title": "Phở", "serves": 2}))
        );
    }

    #[test]
    fn a_bad_write_is_refused_with_its_field_and_reason() {
        let f = fields();
        let no = |data: Value| create(&f, &data).unwrap_err();
        assert_eq!(no(json!({})), refused("title", "is required"));
        assert_eq!(
            no(json!({"title": "x", "titel": "y"})),
            refused("titel", "is not a field of this collection")
        );
        assert_eq!(no(json!({"title": 3})), refused("title", "must be text"));
        assert_eq!(
            no(json!({"title": "abcdefghijk"})),
            refused("title", "must be at most 10 characters")
        );
        assert_eq!(
            no(json!({"title": "x", "serves": 0})),
            refused("serves", "must be at least 1")
        );
        assert_eq!(
            no(json!({"title": "x", "tags": ["a", 2]})),
            refused("tags[1]", "must be text")
        );
        assert_eq!(
            no(json!({"title": "x", "cooked": "6/11/2026"})),
            refused("cooked", "must be a date, YYYY-MM-DD")
        );
        // Ten characters, counted as characters, not bytes.
        assert!(create(&f, &json!({"title": "ở".repeat(10)})).is_ok());
    }

    #[test]
    fn an_update_merges_removes_and_keeps_the_required() {
        let f = fields();
        let held = json!({"title": "Soup", "serves": 4, "tags": ["hot"]});
        assert_eq!(
            update(&f, &held, &json!({"serves": 6, "tags": null})),
            Ok(json!({"title": "Soup", "serves": 6}))
        );
        assert_eq!(
            update(&f, &held, &json!({"title": null})).unwrap_err(),
            refused("title", "is required")
        );
    }

    /// **A record is stored in NFC, made or changed.**
    #[test]
    fn written_text_is_stored_composed() {
        let f = fields();
        let stacked = json!({"title": "Pho\u{031B}\u{0309}"});
        assert_eq!(
            create(&f, &stacked),
            Ok(json!({"title": "Phở", "serves": 2}))
        );
        let held = json!({"title": "Soup"});
        assert_eq!(update(&f, &held, &stacked), Ok(json!({"title": "Phở"})));
    }
}
