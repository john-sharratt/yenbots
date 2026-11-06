//! The query language (§7.2): `where` is `field op value` clauses joined with
//! `and`, compiled to `json_extract` comparisons with bound parameters —
//! never string-built SQL. `order` is a field, `-field` for descending.
//!
//! ```text
//! serves >= 4 and title ~ "soup" and done = false
//! ```
//!
//! Operators: `=`, `!=`, `<`, `<=`, `>`, `>=`, and `~` (contains, any case).
//! A value is a number, `true`, `false`, `null`, or text — quoted when it
//! holds a space.

use std::collections::BTreeMap;

use rusqlite::types::Value as Sql;

use crate::apps::manifest::Field;

/// One comparison.
#[derive(Debug, Clone, PartialEq)]
pub struct Clause {
    pub field: String,
    pub op: &'static str,
    pub value: Sql,
}

const OPS: [&str; 7] = ["<=", ">=", "!=", "=", "<", ">", "~"];

fn value_of(text: &str) -> Sql {
    let t = text.trim();
    if let Some(quoted) = t.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        return Sql::Text(quoted.to_string());
    }
    match t {
        "true" => Sql::Integer(1),
        "false" => Sql::Integer(0),
        "null" => Sql::Null,
        _ => {
            if let Ok(i) = t.parse::<i64>() {
                Sql::Integer(i)
            } else if let Ok(f) = t.parse::<f64>() {
                Sql::Real(f)
            } else {
                Sql::Text(t.to_string())
            }
        }
    }
}

/// `text` as clauses over `fields`, or why it is not a query.
pub fn parse(text: &str, fields: &BTreeMap<String, Field>) -> Result<Vec<Clause>, String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(Vec::new());
    }
    let mut clauses = Vec::new();
    for part in split_and(text) {
        let part = part.trim();
        let (at, op) = OPS
            .iter()
            .filter_map(|op| part.find(op).map(|at| (at, *op)))
            .min_by_key(|(at, op)| (*at, std::cmp::Reverse(op.len())))
            .ok_or_else(|| format!("`{part}` has no operator: = != < <= > >= ~"))?;
        let field = part[..at].trim();
        if !fields.contains_key(field) {
            return Err(format!("`{field}` is not a field of this collection"));
        }
        clauses.push(Clause {
            field: field.to_string(),
            op,
            value: value_of(&part[at + op.len()..]),
        });
    }
    Ok(clauses)
}

/// The clauses of `text`, split on ` and ` outside quotes.
fn split_and(text: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut rest = text;
    while !rest.is_empty() {
        if !quoted && rest.starts_with(" and ") {
            parts.push(std::mem::take(&mut current));
            rest = &rest[5..];
            continue;
        }
        let c = rest.chars().next().expect("not empty");
        if c == '"' {
            quoted = !quoted;
        }
        current.push(c);
        rest = &rest[c.len_utf8()..];
    }
    parts.push(current);
    parts
}

/// The SQL condition for `clauses` and its parameters, after `first` others.
pub fn compile(clauses: &[Clause], first: usize) -> (String, Vec<Sql>) {
    let mut sql = Vec::new();
    let mut params = Vec::new();
    for c in clauses {
        let path = format!("$.\"{}\"", c.field);
        let at = first + params.len() + 1;
        params.push(Sql::Text(path));
        let target = format!("json_extract(data, ?{at})");
        match (c.op, &c.value) {
            ("=", Sql::Null) => sql.push(format!("{target} IS NULL")),
            ("!=", Sql::Null) => sql.push(format!("{target} IS NOT NULL")),
            ("~", v) => {
                let needle = match v {
                    Sql::Text(t) => t.clone(),
                    Sql::Integer(i) => i.to_string(),
                    Sql::Real(f) => f.to_string(),
                    _ => String::new(),
                };
                params.push(Sql::Text(format!("%{needle}%")));
                sql.push(format!("{target} LIKE ?{}", at + 1));
            }
            (op, v) => {
                params.push(v.clone());
                sql.push(format!("{target} {op} ?{}", at + 1));
            }
        }
    }
    if sql.is_empty() {
        ("1".to_string(), params)
    } else {
        (sql.join(" AND "), params)
    }
}

/// The order `text` asks for: a field, and whether descending.
pub fn order(text: &str, fields: &BTreeMap<String, Field>) -> Result<(String, bool), String> {
    let (field, desc) = match text.strip_prefix('-') {
        Some(f) => (f, true),
        None => (text, false),
    };
    if !fields.contains_key(field) {
        return Err(format!("`{field}` is not a field of this collection"));
    }
    Ok((field.to_string(), desc))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields() -> BTreeMap<String, Field> {
        serde_yaml::from_str(
            "title: { type: text }\nserves: { type: number }\ndone: { type: bool }\n",
        )
        .unwrap()
    }

    #[test]
    fn clauses_compile_to_bound_comparisons() {
        let text = r#"serves >= 4 and title ~ "hot soup" and done = false"#;
        let c = parse(text, &fields()).unwrap();
        let clause = |field: &str, op, value| Clause {
            field: field.into(),
            op,
            value,
        };
        assert_eq!(
            c,
            vec![
                clause("serves", ">=", Sql::Integer(4)),
                clause("title", "~", Sql::Text("hot soup".into())),
                clause("done", "=", Sql::Integer(0)),
            ]
        );
        let (sql, params) = compile(&c, 2);
        assert_eq!(
            sql,
            "json_extract(data, ?3) >= ?4 AND json_extract(data, ?5) LIKE ?6 AND json_extract(data, ?7) = ?8"
        );
        assert_eq!(params[1], Sql::Integer(4));
        assert_eq!(params[3], Sql::Text("%hot soup%".into()));
    }

    /// **A field name never reaches the SQL text**: an unknown field is
    /// refused, and a known one is a bound parameter.
    #[test]
    fn nothing_from_the_query_is_spliced_into_sql() {
        assert!(parse("x') OR 1=1 -- = 1", &fields()).is_err());
        let (sql, _) = compile(&parse("title = '; DROP TABLE docs", &fields()).unwrap(), 0);
        assert!(!sql.contains("DROP"));
        let none = parse("serves", &fields()).unwrap_err();
        assert!(none.contains("no operator"));
    }

    #[test]
    fn an_order_is_a_field_up_or_down() {
        assert_eq!(order("-serves", &fields()), Ok(("serves".into(), true)));
        assert!(order("nope", &fields()).is_err());
    }
}
