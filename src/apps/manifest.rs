//! An app's manifest, `apps/<id>/app.yaml` (`candle/docs/yenbot_app_host.md`
//! §5.1, §7.5): what the app is called, whether it is live, and its
//! collections with their fields.

use std::collections::BTreeMap;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

/// Whether an app is on the landing page yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    #[default]
    Draft,
    Live,
}

/// Who a collection's records belong to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    /// Every account that may open the app reads them.
    #[default]
    Shared,
    /// Each record is its owner's alone, unless shared (§12.5).
    Private,
}

/// A field's type (§7.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldType {
    Text,
    Number,
    Bool,
    /// Milliseconds since the epoch.
    Time,
    /// `YYYY-MM-DD`.
    Date,
    /// A file's id.
    File,
    List,
    Object,
    /// Another record's id, in `coll`.
    Ref,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Field {
    #[serde(rename = "type")]
    pub kind: FieldType,
    #[serde(default)]
    pub required: bool,
    /// The least number, or the fewest characters or items.
    #[serde(default)]
    pub min: Option<f64>,
    /// The greatest number, or the most characters or items.
    #[serde(default)]
    pub max: Option<f64>,
    /// Given when a new record leaves the field out.
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    /// A text field shown as a paragraph rather than a line.
    #[serde(default)]
    pub long: bool,
    /// A list's item type.
    #[serde(default)]
    pub of: Option<FieldType>,
    /// A ref's collection.
    #[serde(default)]
    pub coll: Option<String>,
}

/// Who may read and who may write a shared collection (§12.5): `everyone`
/// (every account that may open the app), or group names.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Roles {
    #[serde(default = "everyone")]
    pub read: Vec<String>,
    #[serde(default = "everyone")]
    pub write: Vec<String>,
}

pub const EVERYONE: &str = "everyone";

fn everyone() -> Vec<String> {
    vec![EVERYONE.into()]
}

impl Default for Roles {
    fn default() -> Roles {
        Roles {
            read: everyone(),
            write: everyone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Collection {
    #[serde(default)]
    pub scope: Scope,
    /// The shape's version (§7.5).
    #[serde(default = "one")]
    pub version: u32,
    pub fields: BTreeMap<String, Field>,
    /// What full-text search covers (§7.6).
    #[serde(default)]
    pub search: Vec<String>,
    /// Fields filtered or scheduled on, given an index.
    #[serde(default)]
    pub index: Vec<String>,
    #[serde(default)]
    pub roles: Roles,
}

fn one() -> u32 {
    1
}

/// The manifest's fields this part of the platform reads; the rest of the
/// manifest (tile, schedules, theme, …) is read where it is used.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub status: Status,
    #[serde(default)]
    pub collections: BTreeMap<String, Collection>,
}

/// Whether `name` is one an app, collection or field may have: lowercase
/// letters, digits, `-` and `_`, starting with a letter, at most 48.
pub fn is_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 48
        && name.starts_with(|c: char| c.is_ascii_lowercase())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

impl Manifest {
    /// The manifest in `text`, for the app folder named `folder`, checked.
    pub fn parse(text: &str, folder: &str) -> Result<Manifest> {
        let m: Manifest = serde_yaml::from_str(text).context("app.yaml does not parse")?;
        if m.id != folder {
            bail!("app.yaml names `{}` but its folder is `{folder}`", m.id);
        }
        if !is_name(&m.id) {
            bail!(
                "`{}` is not an app id: lowercase letters, digits, `-` and `_`",
                m.id
            );
        }
        for (name, c) in &m.collections {
            if !is_name(name) {
                bail!("`{name}` is not a collection name");
            }
            for (field, f) in &c.fields {
                if !is_name(field) {
                    bail!("{name}: `{field}` is not a field name");
                }
                if f.kind == FieldType::List && f.of.is_none() {
                    bail!("{name}.{field}: a list says what it holds, `of: text`");
                }
                if f.kind == FieldType::Ref && f.coll.is_none() {
                    bail!("{name}.{field}: a ref names its collection, `coll: …`");
                }
                if let Some(coll) = &f.coll {
                    if !m.collections.contains_key(coll) {
                        bail!("{name}.{field}: no collection `{coll}` to refer to");
                    }
                }
            }
            for listed in c.search.iter().chain(&c.index) {
                if !c.fields.contains_key(listed) {
                    bail!("{name}: `{listed}` is listed but is not a field");
                }
            }
        }
        Ok(m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECIPES: &str = r#"
id: recipes
name: Recipes
status: live
collections:
  recipes:
    fields:
      title:  { type: text, required: true, max: 120 }
      serves: { type: number, min: 1, default: 2 }
      tags:   { type: list, of: text }
      by:     { type: ref, coll: cooks }
    search: [title, tags]
    index: [serves]
  cooks:
    scope: private
    fields:
      name: { type: text }
"#;

    #[test]
    fn a_manifest_reads_its_collections_and_fields() {
        let m = Manifest::parse(RECIPES, "recipes").unwrap();
        assert_eq!((m.name.as_str(), m.status), ("Recipes", Status::Live));
        let r = &m.collections["recipes"];
        assert_eq!((r.scope, r.version), (Scope::Shared, 1));
        assert_eq!(r.fields["title"].max, Some(120.0));
        assert_eq!(r.roles, Roles::default());
        assert_eq!(m.collections["cooks"].scope, Scope::Private);
    }

    #[test]
    fn a_manifest_that_cannot_be_served_is_refused_with_why() {
        let bad = |text: &str, folder: &str| Manifest::parse(text, folder).unwrap_err().to_string();
        assert!(bad(RECIPES, "other").contains("its folder is `other`"));
        let changed = |from: &str, to: &str| bad(&RECIPES.replace(from, to), "recipes");
        assert!(changed("of: text", "max: 3").contains("says what it holds"));
        assert!(changed("coll: cooks", "coll: chefs").contains("no collection `chefs`"));
        let unlisted = changed("search: [title, tags]", "search: [body]");
        assert!(unlisted.contains("`body` is listed but is not a field"));
        assert!(changed("serves: {", "serves: { typo: 1,").contains("parse"));
    }

    #[test]
    fn names_are_lowercase_words() {
        assert!(is_name("recipes"));
        assert!(is_name("shopping-list_2"));
        assert!(!is_name("Recipes"));
        assert!(!is_name("2recipes"));
        assert!(!is_name("a b"));
        assert!(!is_name(""));
    }
}
