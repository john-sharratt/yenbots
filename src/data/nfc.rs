//! Text normalised on every write (§8.7): the same Vietnamese word, typed on
//! a phone that composes `ở` as one character or on one that stacks the
//! horn and the hook, is stored as the same bytes, so it compares, sorts and
//! searches as the same word.

use serde_json::Value;
use unicode_normalization::UnicodeNormalization;

/// `value` with every string in it, keys included, in Unicode NFC.
pub fn nfc(value: Value) -> Value {
    match value {
        Value::String(s) => Value::String(s.nfc().collect()),
        Value::Array(items) => Value::Array(items.into_iter().map(nfc).collect()),
        Value::Object(fields) => Value::Object(
            fields
                .into_iter()
                .map(|(k, v)| (k.nfc().collect(), nfc(v)))
                .collect(),
        ),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// **`o` + horn + hook above is stored as the one character `ở`.**
    #[test]
    fn stacked_marks_are_composed() {
        let stacked = "Pho\u{031B}\u{0309}";
        assert_eq!(stacked.len(), 7);
        let stored = nfc(json!({ "title": stacked, "tags": [stacked], "n": 2 }));
        assert_eq!(stored, json!({ "title": "Phở", "tags": ["Phở"], "n": 2 }));
        assert_eq!(
            stored["title"].as_str().unwrap().as_bytes(),
            b"Ph\xe1\xbb\x9f"
        );
    }
}
