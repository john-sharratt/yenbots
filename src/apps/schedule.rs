//! A manifest's `schedules:` (§9.2): each runs one workflow, and is one of
//! three kinds —
//!
//! ```yaml
//! schedules:
//!   - every: "weekly sun 19:00"   # a repeat rule, in Yen's time zone
//!     run: plan-meals
//!     catch_up: true              # run once if the site was down when it was due
//!   - from: reminders             # one job per record,
//!     at: due_ms                  #   at this time field,
//!     before: 10m                 #   less this
//!     run: remind
//!   - when: { collection: chores, does: update, where: { done: true } }
//!     run: chore-done             # a write matching the filter
//! ```
//!
//! A `when:` on `update` runs when a record comes to match — it did not
//! before the write and does after — so a chore marked done runs it once,
//! and an edit to a chore already done does not. On `create` it is the new
//! record that matches, on `delete` the record as it was.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::manifest::{Collection, FieldType};
use crate::jobs::rule::Rule;
use crate::jobs::span;

/// The writes a `when:` hears.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Does {
    Create,
    Update,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct When {
    pub collection: String,
    pub does: Does,
    /// Fields and the values they must hold; empty matches every record.
    #[serde(default, rename = "where", skip_serializing_if = "BTreeMap::is_empty")]
    pub filter: BTreeMap<String, Value>,
}

impl When {
    /// Whether the record `data` matches the filter.
    pub fn matches(&self, data: &Value) -> bool {
        self.filter
            .iter()
            .all(|(field, want)| data.get(field).unwrap_or(&Value::Null) == want)
    }
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// One entry of `schedules:`, as written.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Schedule {
    /// The workflow, a file in `workflows/`.
    pub run: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub every: Option<String>,
    /// Run once at start if it fell due while the site was down more than
    /// an hour (§9.4).
    #[serde(default, skip_serializing_if = "is_false")]
    pub catch_up: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<When>,
}

/// What a schedule makes jobs from.
#[derive(Debug, Clone, PartialEq)]
pub enum Kind<'a> {
    Every(Rule),
    /// One job per record of `coll`, at its field `at` less `before_ms`.
    From {
        coll: &'a str,
        at: &'a str,
        before_ms: i64,
    },
    When(&'a When),
}

impl Schedule {
    /// What it makes jobs from, read against the app's `collections`; why
    /// not, when it cannot be read.
    pub fn kind<'a>(
        &'a self,
        collections: &BTreeMap<String, Collection>,
    ) -> Result<Kind<'a>, String> {
        let run = &self.run;
        let given = [
            self.every.is_some(),
            self.from.is_some(),
            self.when.is_some(),
        ];
        if given.iter().filter(|g| **g).count() != 1 {
            return Err(format!(
                "the schedule running `{run}` is one of `every:`, `from:` or `when:`"
            ));
        }
        if self.catch_up && self.every.is_none() {
            return Err(format!(
                "the schedule running `{run}`: `catch_up` is an `every:`'s"
            ));
        }
        if self.from.is_none() && (self.at.is_some() || self.before.is_some()) {
            return Err(format!(
                "the schedule running `{run}`: `at` and `before` are a `from:`'s"
            ));
        }
        if let Some(every) = &self.every {
            return Rule::parse(every)
                .map(Kind::Every)
                .map_err(|e| format!("the schedule running `{run}`: {e}"));
        }
        if let Some(coll) = &self.from {
            let c = collections.get(coll).ok_or_else(|| {
                format!("the schedule running `{run}` is from `{coll}`, which is not a collection")
            })?;
            let at = self.at.as_deref().ok_or_else(|| {
                format!("the schedule running `{run}` names the time field it is `at:`")
            })?;
            if c.fields.get(at).map(|f| f.kind) != Some(FieldType::Time) {
                return Err(format!(
                    "the schedule running `{run}` is at `{coll}.{at}`, which is not a `time` field"
                ));
            }
            let before_ms = match &self.before {
                Some(b) => {
                    span::parse(b).map_err(|e| format!("the schedule running `{run}`: {e}"))?
                }
                None => 0,
            };
            return Ok(Kind::From {
                coll,
                at,
                before_ms,
            });
        }
        let when = self.when.as_ref().expect("one of the three is given");
        let c = collections.get(&when.collection).ok_or_else(|| {
            format!(
                "the schedule running `{run}` is on `{}`, which is not a collection",
                when.collection
            )
        })?;
        for field in when.filter.keys() {
            if !c.fields.contains_key(field) {
                return Err(format!(
                    "the schedule running `{run}` filters on `{}.{field}`, which is not a field",
                    when.collection
                ));
            }
        }
        Ok(Kind::When(when))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn collections() -> BTreeMap<String, Collection> {
        serde_yaml::from_str(
            "reminders:\n  fields:\n    title: { type: text }\n    due_ms: { type: time }\n\
             chores:\n  fields:\n    done: { type: bool }\n",
        )
        .unwrap()
    }

    fn read(yaml: &str) -> Result<String, String> {
        let s: Schedule = serde_yaml::from_str(yaml).unwrap();
        s.kind(&collections()).map(|k| format!("{k:?}"))
    }

    #[test]
    fn each_kind_of_schedule_is_read() {
        assert_eq!(
            read("every: daily 07:30\nrun: m"),
            Ok("Every(Daily(7, 30))".into())
        );
        assert_eq!(
            read("from: reminders\nat: due_ms\nbefore: 10m\nrun: remind"),
            Ok("From { coll: \"reminders\", at: \"due_ms\", before_ms: 600000 }".into())
        );
        let s: Schedule = serde_yaml::from_str(
            "when: {collection: chores, does: update, where: {done: true}}\nrun: d",
        )
        .unwrap();
        let Ok(Kind::When(w)) = s.kind(&collections()) else {
            panic!("a when");
        };
        assert_eq!(w.does, Does::Update);
        assert!(w.matches(&json!({"done": true})));
        assert!(!w.matches(&json!({"done": false})));
        assert!(!w.matches(&json!({})));
    }

    #[test]
    fn a_schedule_that_cannot_be_read_is_refused_with_why() {
        let told = |yaml: &str| read(yaml).unwrap_err();
        assert_eq!(
            told("run: m"),
            "the schedule running `m` is one of `every:`, `from:` or `when:`"
        );
        assert_eq!(
            told("every: hourly\nfrom: reminders\nrun: m"),
            "the schedule running `m` is one of `every:`, `from:` or `when:`"
        );
        assert_eq!(
            told("every: daily 7am\nrun: m"),
            "the schedule running `m`: `7am` is not a time of day: `HH:MM`, as `07:30` or `19:00`"
        );
        assert_eq!(
            told("from: notes\nat: due_ms\nrun: m"),
            "the schedule running `m` is from `notes`, which is not a collection"
        );
        assert_eq!(
            told("from: reminders\nat: title\nrun: m"),
            "the schedule running `m` is at `reminders.title`, which is not a `time` field"
        );
        assert_eq!(
            told("from: reminders\nrun: m"),
            "the schedule running `m` names the time field it is `at:`"
        );
        assert_eq!(
            told("from: reminders\nat: due_ms\nbefore: soon\nrun: m"),
            "the schedule running `m`: `soon` is not a span: a number and `s`, `m`, `h` or `d`, as `10m`"
        );
        assert_eq!(
            told("when: {collection: chores, does: update, where: {finished: true}}\nrun: m"),
            "the schedule running `m` filters on `chores.finished`, which is not a field"
        );
        assert_eq!(
            told("every: hourly\nat: due_ms\nrun: m"),
            "the schedule running `m`: `at` and `before` are a `from:`'s"
        );
        assert_eq!(
            told("from: reminders\nat: due_ms\ncatch_up: true\nrun: m"),
            "the schedule running `m`: `catch_up` is an `every:`'s"
        );
    }
}
