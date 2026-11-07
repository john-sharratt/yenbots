//! An answer held to the shape an app asked for (§14): the parts of JSON
//! Schema an app's answer shapes use — `type`, `properties`, `required`,
//! `items`, `enum` — checked; any other keyword is the model's to follow and
//! is not held.

use serde_json::Value;

/// Whether `t` names `v`'s JSON type.
fn is_type(v: &Value, t: &str) -> bool {
    match t {
        "string" => v.is_string(),
        "number" => v.is_number(),
        "integer" => v.is_i64() || v.is_u64(),
        "boolean" => v.is_boolean(),
        "object" => v.is_object(),
        "array" => v.is_array(),
        "null" => v.is_null(),
        _ => true,
    }
}

/// Whether `v`, found at `at`, fits `schema`; why not, when it does not.
pub fn fits(v: &Value, schema: &Value, at: &str) -> Result<(), String> {
    match schema.get("type") {
        Some(Value::String(t)) if !is_type(v, t) => {
            let a = if matches!(t.as_str(), "object" | "array" | "integer") {
                "an"
            } else {
                "a"
            };
            return Err(format!("{at} is not {a} {t}"));
        }
        Some(Value::Array(ts)) if !ts.iter().filter_map(Value::as_str).any(|t| is_type(v, t)) => {
            return Err(format!("{at} is none of the types it may be"))
        }
        _ => {}
    }
    if let Some(Value::Array(allowed)) = schema.get("enum") {
        if !allowed.contains(v) {
            return Err(format!("{at} is not one of the values it may be"));
        }
    }
    if let Value::Object(o) = v {
        if let Some(Value::Array(required)) = schema.get("required") {
            for r in required.iter().filter_map(Value::as_str) {
                if !o.contains_key(r) {
                    return Err(format!("{at} has no `{r}`"));
                }
            }
        }
        if let Some(Value::Object(props)) = schema.get("properties") {
            for (k, s) in props {
                if let Some(inner) = o.get(k) {
                    fits(inner, s, &format!("{at}.{k}"))?;
                }
            }
        }
    }
    if let (Value::Array(items), Some(s)) = (v, schema.get("items")) {
        for (i, item) in items.iter().enumerate() {
            fits(item, s, &format!("{at}[{i}]"))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn an_answer_is_held_to_its_shape() {
        let schema = json!({
            "type": "object",
            "required": ["meals"],
            "properties": {
                "meals": {"type": "array", "items": {
                    "type": "object", "required": ["day", "dish"],
                    "properties": {"day": {"enum": ["mon", "tue"]}, "dish": {"type": "string"}, "serves": {"type": "integer"}}
                }}
            }
        });
        let ok = json!({"meals": [{"day": "mon", "dish": "Phở", "serves": 4}]});
        assert_eq!(fits(&ok, &schema, "the answer"), Ok(()));
        let told = |v: Value| fits(&v, &schema, "the answer").unwrap_err();
        assert_eq!(told(json!([])), "the answer is not an object");
        assert_eq!(told(json!({})), "the answer has no `meals`");
        assert_eq!(
            told(json!({"meals": [{"day": "sun", "dish": "x"}]})),
            "the answer.meals[0].day is not one of the values it may be"
        );
        assert_eq!(
            told(json!({"meals": [{"day": "mon", "dish": "x", "serves": 2.5}]})),
            "the answer.meals[0].serves is not an integer"
        );
        assert_eq!(
            fits(&json!(null), &json!({"type": ["string", "null"]}), "x"),
            Ok(())
        );
    }
}
