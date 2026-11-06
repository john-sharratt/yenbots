//! The apps' records (§7.2, §7.6, §12.5): one document table for every app,
//! each record its collection's, owned by the account that made it.
//!
//! **Access is decided here, never by the request.** A private collection's
//! records are their owner's, and those shared with the caller or a group of
//! theirs; a shared collection's are read and written by the accounts its
//! `roles` name (`everyone`, or groups). An admin reads everything through
//! the admin routes alone.

use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::Result;
use rusqlite::types::Value as Sql;
use rusqlite::{params, params_from_iter, Connection, OptionalExtension, Row, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use super::query::{self, compile};
use super::search::{body_of, fold, index, unindex};
use super::validate::{self, Refused};
use crate::accounts::caller::Caller;
use crate::apps::manifest::{Collection, Manifest, Scope, EVERYONE};

/// One record, as the API answers it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Doc {
    pub id: String,
    pub owner: String,
    pub data: Value,
    pub version: i64,
    pub created_ms: i64,
    pub updated_ms: i64,
}

/// Why a call on the records did not do what it asked.
#[derive(Debug, Clone, PartialEq)]
pub enum Denied {
    /// No such app or collection.
    NoCollection(String),
    /// No such record, or none the caller may see.
    NotFound,
    /// The caller may see it but not do this.
    Forbidden(&'static str),
    /// The record moved on since the caller read it.
    Stale { held: i64 },
    /// The write does not fit the collection.
    Invalid(Refused),
    /// The request is not one the store can answer.
    Bad(String),
}

pub type Answer<T> = std::result::Result<T, Denied>;

/// A batch's outcome: every write's result, or the place of the first
/// refused and why.
pub type Batched = std::result::Result<Vec<Wrote>, (usize, Denied)>;

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A new record's id: its time, then a counter, so ids sort by creation.
pub fn new_id(now_ms: i64) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let salt = std::process::id() as u64;
    let count = n.wrapping_add(salt << 20) & 0xf_ffff_ffff;
    format!("{now_ms:011x}{count:09x}")
}

/// The collection `coll` of `app`, or why not.
pub fn collection<'m>(app: &'m Manifest, coll: &str) -> Answer<&'m Collection> {
    app.collections
        .get(coll)
        .ok_or_else(|| Denied::NoCollection(format!("{} has no collection `{coll}`", app.id)))
}

/// Whether a role list — `everyone`, or group names — names `caller`.
pub fn admits(names: &[String], caller: &Caller) -> bool {
    names.iter().any(|n| n == EVERYONE) || caller.groups.iter().any(|g| names.contains(g))
}

/// The share rows `caller` matches, as an SQL condition's parameters.
fn principals(caller: &Caller) -> String {
    serde_json::to_string(&caller.principals()).expect("strings serialize")
}

fn row(r: &Row<'_>) -> rusqlite::Result<Doc> {
    let data: String = r.get(2)?;
    Ok(Doc {
        id: r.get(0)?,
        owner: r.get(1)?,
        data: serde_json::from_str(&data).unwrap_or(Value::Null),
        version: r.get(3)?,
        created_ms: r.get(4)?,
        updated_ms: r.get(5)?,
    })
}

const COLUMNS: &str = "id, owner, data, version, created_ms, updated_ms";

fn bad(why: impl Into<String>) -> Result<Answer<()>> {
    Ok(Err(Denied::Bad(why.into())))
}

/// Whether `caller` may read rows of `c` — `?1` the caller's id and `?2`
/// their principals as JSON — as an SQL condition.
fn visible(c: &Collection, caller: &Caller) -> Option<&'static str> {
    match c.scope {
        Scope::Private => Some(
            "(owner = ?1 OR EXISTS (SELECT 1 FROM shares s WHERE s.app = docs.app AND \
             s.coll = docs.coll AND s.id = docs.id AND s.who IN (SELECT value FROM json_each(?2))))",
        ),
        Scope::Shared if admits(&c.roles.read, caller) => Some("1"),
        Scope::Shared => None,
    }
}

/// Where a record is: its app, its collection and the collection's rules.
struct Place<'a> {
    app: &'a str,
    coll: &'a str,
    c: &'a Collection,
}

/// Whether `caller` may change `doc`.
fn may_edit(conn: &Connection, at: &Place<'_>, caller: &Caller, doc: &Doc) -> Result<bool> {
    if doc.owner == caller.id() {
        return Ok(true);
    }
    let shared: bool = conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM shares WHERE app = ?1 AND coll = ?2 AND id = ?3 AND edit = 1 \
         AND who IN (SELECT value FROM json_each(?4)))",
        params![at.app, at.coll, doc.id, principals(caller)],
        |r| r.get(0),
    )?;
    Ok(shared || (at.c.scope == Scope::Shared && admits(&at.c.roles.write, caller)))
}

fn load(conn: &Connection, at: &Place<'_>, caller: &Caller, id: &str) -> Result<Answer<Doc>> {
    let Some(cond) = visible(at.c, caller) else {
        return Ok(Err(Denied::NotFound));
    };
    let sql =
        format!("SELECT {COLUMNS} FROM docs WHERE app = ?3 AND coll = ?4 AND id = ?5 AND {cond}");
    let args = params![caller.id(), principals(caller), at.app, at.coll, id];
    Ok(conn
        .query_row(&sql, args, row)
        .optional()?
        .ok_or(Denied::NotFound))
}

/// The place of `coll` in `app`, or why there is none.
fn place<'a>(app: &'a Manifest, coll: &'a str) -> Answer<Place<'a>> {
    Ok(Place {
        app: &app.id,
        coll,
        c: collection(app, coll)?,
    })
}

/// Make a record of `coll` from `data`.
pub fn create(
    tx: &Transaction<'_>,
    app: &Manifest,
    coll: &str,
    caller: &Caller,
    data: &Value,
    now: i64,
) -> Result<Answer<Doc>> {
    let c = match collection(app, coll) {
        Ok(c) => c,
        Err(d) => return Ok(Err(d)),
    };
    if c.scope == Scope::Shared && !admits(&c.roles.write, caller) {
        let why = "this collection is not yours to add to";
        return Ok(Err(Denied::Forbidden(why)));
    }
    let data = match validate::create(&c.fields, data) {
        Ok(d) => d,
        Err(r) => return Ok(Err(Denied::Invalid(r))),
    };
    let doc = Doc {
        id: new_id(now),
        owner: caller.id().to_string(),
        data,
        version: 1,
        created_ms: now,
        updated_ms: now,
    };
    tx.execute(
        "INSERT INTO docs (app, coll, id, owner, data, version, created_ms, updated_ms) \
         VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?6)",
        params![app.id, coll, doc.id, doc.owner, doc.data.to_string(), now],
    )?;
    index(tx, &app.id, coll, &doc.id, &body_of(&c.search, &doc.data))?;
    Ok(Ok(doc))
}

pub fn get(
    conn: &Connection,
    app: &Manifest,
    coll: &str,
    caller: &Caller,
    id: &str,
) -> Result<Answer<Doc>> {
    match place(app, coll) {
        Ok(at) => load(conn, &at, caller, id),
        Err(d) => Ok(Err(d)),
    }
}

/// An update: the fields merged in, and the version the writer read.
#[derive(Debug, Clone, Copy)]
pub struct Change<'a> {
    pub patch: &'a Value,
    /// When given, the record must still be at this version.
    pub expect: Option<i64>,
}

/// Merge `change` into record `id`.
pub fn update(
    tx: &Transaction<'_>,
    app: &Manifest,
    coll: &str,
    caller: &Caller,
    id: &str,
    change: Change<'_>,
    now: i64,
) -> Result<Answer<Doc>> {
    let Change { patch, expect } = change;
    let at = match place(app, coll) {
        Ok(at) => at,
        Err(d) => return Ok(Err(d)),
    };
    let held = match load(tx, &at, caller, id)? {
        Ok(d) => d,
        Err(d) => return Ok(Err(d)),
    };
    if !may_edit(tx, &at, caller, &held)? {
        return Ok(Err(Denied::Forbidden("this record is not yours to change")));
    }
    if let Some(v) = expect {
        if v != held.version {
            return Ok(Err(Denied::Stale { held: held.version }));
        }
    }
    let data = match validate::update(&at.c.fields, &held.data, patch) {
        Ok(d) => d,
        Err(r) => return Ok(Err(Denied::Invalid(r))),
    };
    tx.execute(
        "UPDATE docs SET data = ?4, version = version + 1, updated_ms = ?5 \
         WHERE app = ?1 AND coll = ?2 AND id = ?3",
        params![app.id, coll, id, data.to_string(), now],
    )?;
    unindex(tx, &app.id, coll, id)?;
    index(tx, &app.id, coll, id, &body_of(&at.c.search, &data))?;
    Ok(Ok(Doc {
        data,
        version: held.version + 1,
        updated_ms: now,
        ..held
    }))
}

pub fn delete(
    tx: &Transaction<'_>,
    app: &Manifest,
    coll: &str,
    caller: &Caller,
    id: &str,
) -> Result<Answer<()>> {
    let at = match place(app, coll) {
        Ok(at) => at,
        Err(d) => return Ok(Err(d)),
    };
    let held = match load(tx, &at, caller, id)? {
        Ok(d) => d,
        Err(d) => return Ok(Err(d)),
    };
    if !may_edit(tx, &at, caller, &held)? {
        return Ok(Err(Denied::Forbidden("this record is not yours to delete")));
    }
    tx.execute(
        "DELETE FROM docs WHERE app = ?1 AND coll = ?2 AND id = ?3",
        params![app.id, coll, id],
    )?;
    tx.execute(
        "DELETE FROM shares WHERE app = ?1 AND coll = ?2 AND id = ?3",
        params![app.id, coll, id],
    )?;
    unindex(tx, &app.id, coll, id)?;
    Ok(Ok(()))
}

/// A query's arguments, as the route takes them.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Query {
    #[serde(default, rename = "where")]
    pub filter: Option<String>,
    #[serde(default)]
    pub order: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
    /// The cursor the last page answered.
    #[serde(default)]
    pub after: Option<String>,
}

/// One page of records.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Page {
    pub docs: Vec<Doc>,
    /// Ask for the next page with this; absent on the last.
    pub after: Option<String>,
}

pub const MAX_PAGE: usize = 200;

fn hex(s: &str) -> String {
    s.bytes().map(|b| format!("{b:02x}")).collect()
}

fn unhex(s: &str) -> Option<String> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    let bytes: Option<Vec<u8>> = (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect();
    String::from_utf8(bytes?).ok()
}

/// The `[key, id]` a cursor holds, or `None` for one this site did not give.
fn cursor(after: &str) -> Option<(Value, String)> {
    let Value::Array(at) = serde_json::from_str(&unhex(after)?).ok()? else {
        return None;
    };
    match (at.first(), at.get(1)) {
        (Some(v), Some(Value::String(id))) => Some((v.clone(), id.clone())),
        _ => None,
    }
}

/// The caller's id, their principals, the app and the collection: `?1` to
/// `?4` of every query here.
fn base_params(caller: &Caller, app: &Manifest, coll: &str) -> Vec<Sql> {
    vec![
        Sql::Text(caller.id().to_string()),
        Sql::Text(principals(caller)),
        Sql::Text(app.id.clone()),
        Sql::Text(coll.to_string()),
    ]
}

/// A page of `coll`'s records matching `q`, in its order.
pub fn query(
    conn: &Connection,
    app: &Manifest,
    coll: &str,
    caller: &Caller,
    q: &Query,
    all: bool,
) -> Result<Answer<Page>> {
    let c = match collection(app, coll) {
        Ok(c) => c,
        Err(d) => return Ok(Err(d)),
    };
    let cond = match (all, visible(c, caller)) {
        (true, _) => "1",
        (false, Some(cond)) => cond,
        (false, None) => {
            let none = Page {
                docs: Vec::new(),
                after: None,
            };
            return Ok(Ok(none));
        }
    };
    let clauses = match query::parse(q.filter.as_deref().unwrap_or(""), &c.fields) {
        Ok(cl) => cl,
        Err(e) => return Ok(Err(Denied::Bad(e))),
    };
    let order = q.order.as_deref().map(|o| query::order(o, &c.fields));
    let order = match order.transpose() {
        Ok(o) => o,
        Err(e) => return Ok(Err(Denied::Bad(e))),
    };
    let mut params = base_params(caller, app, coll);
    let (filter, more) = compile(&clauses, params.len());
    params.extend(more);
    let (key, desc) = match &order {
        Some((field, desc)) => {
            params.push(Sql::Text(format!("$.\"{field}\"")));
            (format!("json_extract(data, ?{})", params.len()), *desc)
        }
        None => ("created_ms".to_string(), false),
    };
    let (cmp, dir) = if desc { ("<", "DESC") } else { (">", "ASC") };
    let mut seek = String::from("1");
    if let Some(after) = &q.after {
        let Some((v, id)) = cursor(after) else {
            let why = "`after` is not a cursor this site gave";
            return Ok(Err(Denied::Bad(why.into())));
        };
        params.push(json_to_sql(&v));
        let vat = params.len();
        params.push(Sql::Text(id));
        let iat = params.len();
        seek = format!("({key} {cmp} ?{vat} OR ({key} = ?{vat} AND id {cmp} ?{iat}))");
    }
    let limit = q.limit.unwrap_or(50).clamp(1, MAX_PAGE);
    let sql = format!(
        "SELECT {COLUMNS}, {key} FROM docs \
         WHERE app = ?3 AND coll = ?4 AND {cond} AND {filter} AND {seek} \
         ORDER BY {key} {dir}, id {dir} LIMIT {}",
        limit + 1
    );
    let mut stmt = conn.prepare(&sql)?;
    let mut keyed: Vec<(Doc, Value)> = stmt
        .query_map(params_from_iter(params.iter()), |r| {
            let k: Sql = r.get(6)?;
            Ok((row(r)?, sql_to_json(k)))
        })?
        .collect::<Result<_, _>>()?;
    let after = (keyed.len() > limit).then(|| {
        keyed.truncate(limit);
        let (last, k) = keyed.last().expect("a full page");
        hex(&json!([k, last.id]).to_string())
    });
    Ok(Ok(Page {
        docs: keyed.into_iter().map(|(d, _)| d).collect(),
        after,
    }))
}

fn json_to_sql(v: &Value) -> Sql {
    match v {
        Value::Null => Sql::Null,
        Value::Bool(b) => Sql::Integer(i64::from(*b)),
        Value::Number(n) => match n.as_i64() {
            Some(i) => Sql::Integer(i),
            None => Sql::Real(n.as_f64().unwrap_or(0.0)),
        },
        Value::String(s) => Sql::Text(s.clone()),
        other => Sql::Text(other.to_string()),
    }
}

fn sql_to_json(v: Sql) -> Value {
    match v {
        Sql::Null => Value::Null,
        Sql::Integer(i) => json!(i),
        Sql::Real(f) => json!(f),
        Sql::Text(s) => json!(s),
        Sql::Blob(_) => Value::Null,
    }
}

/// What a sum asks for (§7.6).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Sum {
    #[serde(default, rename = "where")]
    pub filter: Option<String>,
    /// One or two fields, comma-separated.
    #[serde(default)]
    pub group: Option<String>,
    /// The field `sum`, `avg`, `min` and `max` are taken over.
    #[serde(default)]
    pub of: Option<String>,
}

/// The fields a sum groups by, refused when they are more than two or not
/// the collection's.
fn sum_groups<'s>(s: &'s Sum, c: &Collection) -> std::result::Result<Vec<&'s str>, String> {
    let groups: Vec<&str> = match s.group.as_deref() {
        Some(g) => g
            .split(',')
            .map(str::trim)
            .filter(|g| !g.is_empty())
            .collect(),
        None => Vec::new(),
    };
    if groups.len() > 2 {
        return Err("a sum groups by one or two fields".into());
    }
    for f in groups.iter().chain(s.of.as_deref().iter()) {
        if !c.fields.contains_key(*f) {
            return Err(format!("`{f}` is not a field of this collection"));
        }
    }
    Ok(groups)
}

/// Counts — and sums, averages, least and greatest of `of` — grouped.
pub fn sum(
    conn: &Connection,
    app: &Manifest,
    coll: &str,
    caller: &Caller,
    s: &Sum,
) -> Result<Answer<Vec<Value>>> {
    let c = match collection(app, coll) {
        Ok(c) => c,
        Err(d) => return Ok(Err(d)),
    };
    let Some(cond) = visible(c, caller) else {
        return Ok(Ok(Vec::new()));
    };
    let clauses = match query::parse(s.filter.as_deref().unwrap_or(""), &c.fields) {
        Ok(cl) => cl,
        Err(e) => return Ok(Err(Denied::Bad(e))),
    };
    let groups = match sum_groups(s, c) {
        Ok(g) => g,
        Err(e) => return Ok(Err(Denied::Bad(e))),
    };
    let mut params = base_params(caller, app, coll);
    let (filter, more) = compile(&clauses, params.len());
    params.extend(more);
    let mut keys = Vec::new();
    for g in &groups {
        params.push(Sql::Text(format!("$.\"{g}\"")));
        keys.push(format!("json_extract(data, ?{})", params.len()));
    }
    let of = match &s.of {
        Some(f) => {
            params.push(Sql::Text(format!("$.\"{f}\"")));
            let at = format!("json_extract(data, ?{})", params.len());
            format!(", SUM({at}), AVG({at}), MIN({at}), MAX({at})")
        }
        None => String::new(),
    };
    let (select_keys, group_by) = if keys.is_empty() {
        (String::new(), String::new())
    } else {
        let keys = keys.join(", ");
        (format!("{keys}, "), format!(" GROUP BY {keys}"))
    };
    let sql = format!(
        "SELECT {select_keys}COUNT(*){of} FROM docs \
         WHERE app = ?3 AND coll = ?4 AND {cond} AND {filter}{group_by}"
    );
    let mut stmt = conn.prepare(&sql)?;
    let n = groups.len();
    let rows = stmt.query_map(params_from_iter(params.iter()), |r| {
        let mut out = Map::new();
        for (i, g) in groups.iter().enumerate() {
            out.insert((*g).to_string(), sql_to_json(r.get(i)?));
        }
        out.insert("count".into(), json!(r.get::<_, i64>(n)?));
        if s.of.is_some() {
            for (i, k) in ["sum", "avg", "min", "max"].iter().enumerate() {
                out.insert((*k).into(), sql_to_json(r.get(n + 1 + i)?));
            }
        }
        Ok(Value::Object(out))
    })?;
    Ok(Ok(rows.collect::<Result<_, _>>()?))
}

/// The records of `coll` whose searchable fields hold `q`, best first.
pub fn search(
    conn: &Connection,
    app: &Manifest,
    coll: &str,
    caller: &Caller,
    q: &str,
    limit: usize,
) -> Result<Answer<Vec<Doc>>> {
    let c = match collection(app, coll) {
        Ok(c) => c,
        Err(d) => return Ok(Err(d)),
    };
    if c.search.is_empty() {
        let why = format!("`{coll}` names no fields to search");
        return Ok(Err(Denied::Bad(why)));
    }
    let Some(cond) = visible(c, caller) else {
        return Ok(Ok(Vec::new()));
    };
    // Each word a prefix term, quoted: nothing the visitor types is FTS syntax.
    let terms: Vec<String> = fold(q)
        .split_whitespace()
        .map(|w| format!("\"{}\"*", w.replace('"', "")))
        .collect();
    if terms.is_empty() {
        return Ok(Ok(Vec::new()));
    }
    let sql = format!(
        "SELECT docs.id, docs.owner, docs.data, docs.version, docs.created_ms, docs.updated_ms \
         FROM docs JOIN docs_fts f ON f.app = docs.app AND f.coll = docs.coll AND f.id = docs.id \
         WHERE docs.app = ?3 AND docs.coll = ?4 AND {cond} AND docs_fts MATCH ?5 \
         ORDER BY f.rank LIMIT ?6"
    );
    let mut stmt = conn.prepare(&sql)?;
    let limit = limit.clamp(1, MAX_PAGE) as i64;
    let args = params![
        caller.id(),
        principals(caller),
        app.id,
        coll,
        terms.join(" "),
        limit
    ];
    let rows = stmt.query_map(args, row)?;
    Ok(Ok(rows.collect::<Result<_, _>>()?))
}

/// One write of a batch.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Write {
    Create {
        coll: String,
        data: Value,
    },
    Update {
        coll: String,
        id: String,
        data: Value,
        #[serde(default)]
        version: Option<i64>,
    },
    Delete {
        coll: String,
        id: String,
    },
}

/// What one write of a batch did.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Wrote {
    Doc(Doc),
    Deleted { deleted: String },
}

/// Every write of `writes`, or none: the first refused names its place.
pub fn batch(
    tx: &Transaction<'_>,
    app: &Manifest,
    caller: &Caller,
    writes: &[Write],
    now: i64,
) -> Result<Batched> {
    let mut out = Vec::new();
    for (i, w) in writes.iter().enumerate() {
        let done = match w {
            Write::Create { coll, data } => {
                create(tx, app, coll, caller, data, now)?.map(Wrote::Doc)
            }
            Write::Update {
                coll,
                id,
                data,
                version,
            } => {
                let change = Change {
                    patch: data,
                    expect: *version,
                };
                update(tx, app, coll, caller, id, change, now)?.map(Wrote::Doc)
            }
            Write::Delete { coll, id } => {
                let deleted = id.clone();
                delete(tx, app, coll, caller, id)?.map(|()| Wrote::Deleted { deleted })
            }
        };
        match done {
            Ok(w) => out.push(w),
            Err(d) => return Ok(Err((i, d))),
        }
    }
    Ok(Ok(out))
}

/// Who record `id` is shared with, set whole by its owner (§12.5).
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
pub struct Shares {
    /// `account:<id>` or `group:<name>`, to read.
    #[serde(default)]
    pub read: Vec<String>,
    /// To read and edit.
    #[serde(default)]
    pub edit: Vec<String>,
}

pub fn share(
    tx: &Transaction<'_>,
    app: &Manifest,
    coll: &str,
    caller: &Caller,
    id: &str,
    to: &Shares,
) -> Result<Answer<()>> {
    let at = match place(app, coll) {
        Ok(at) => at,
        Err(d) => return Ok(Err(d)),
    };
    let held = match load(tx, &at, caller, id)? {
        Ok(d) => d,
        Err(d) => return Ok(Err(d)),
    };
    if held.owner != caller.id() {
        return Ok(Err(Denied::Forbidden("only its owner shares a record")));
    }
    for who in to.read.iter().chain(&to.edit) {
        if !(who.starts_with("account:") || who.starts_with("group:")) {
            return bad(format!("`{who}` is not `account:<id>` or `group:<name>`"));
        }
    }
    tx.execute(
        "DELETE FROM shares WHERE app = ?1 AND coll = ?2 AND id = ?3",
        params![app.id, coll, id],
    )?;
    let put = |who: &str, edit: bool| {
        tx.execute(
            "INSERT OR REPLACE INTO shares (app, coll, id, who, edit) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![app.id, coll, id, who, edit],
        )
    };
    for who in &to.read {
        put(who, false)?;
    }
    for who in &to.edit {
        put(who, true)?;
    }
    Ok(Ok(()))
}

#[cfg(test)]
#[path = "docs_tests.rs"]
mod tests;
