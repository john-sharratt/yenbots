//! The platform's side of `ctx` (`ctx.js`, §10.2): each call a workflow
//! makes, answered as `{ok}` or `{error, detail}`. Records are read and
//! written through the same code the API's routes use — access judged for
//! the run's caller, each write's jobs kept in its transaction, each write
//! told to the open pages once committed.

use std::sync::Arc;

use anyhow::Result;
use jiff::tz::TimeZone;
use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value};

use super::loader::Sources;
use crate::accounts::caller::Caller;
use crate::accounts::now_ms;
use crate::apps::manifest::{is_name, Manifest};
use crate::data::changes::{change_of, Changes, Feed, Op};
use crate::data::docs::{self, Change, Denied, Doc, Query, Sum};
use crate::jobs::clock::spoken;
use crate::jobs::follow::acting;
use crate::jobs::table::{insert, NewJob};
use crate::store::Store;

/// The most lines a run's log keeps, and the longest each may be.
pub const LOG_LINES: usize = 200;
pub const LOG_LINE: usize = 2_000;

/// What a run reaches, and what it did besides its answer.
pub struct Host {
    pub store: Arc<Store>,
    pub changes: Changes,
    pub manifest: Arc<Manifest>,
    pub caller: Caller,
    pub sources: Sources,
    pub zone: TimeZone,
    /// The run's log, as `ctx.log` wrote it.
    pub log: Vec<String>,
    /// Whether it made a job, so the scheduler looks again.
    pub scheduled: bool,
}

fn refused(code: &str, detail: impl Into<String>) -> Value {
    json!({"error": code, "detail": detail.into()})
}

fn denied(d: Denied) -> Value {
    match d {
        Denied::NoCollection(why) => refused("no_collection", why),
        Denied::NotFound => refused("not_found", "no such record"),
        Denied::Forbidden(why) => refused("forbidden", why),
        Denied::Stale { held } => refused(
            "stale",
            format!("the record changed since it was read; it is at version {held}"),
        ),
        Denied::Invalid(r) => json!({
            "error": "invalid",
            "detail": format!("{} {}", r.field, r.reason),
            "field": r.field,
        }),
        Denied::Bad(why) => refused("bad_request", why),
    }
}

fn str_arg(args: &[Value], i: usize) -> Option<&str> {
    args.get(i).and_then(Value::as_str)
}

/// A query object as the data route reads its query string.
fn query_of(v: Option<&Value>) -> Query {
    let get = |k: &str| v.and_then(|v| v.get(k));
    Query {
        filter: get("where").and_then(Value::as_str).map(str::to_string),
        order: get("order").and_then(Value::as_str).map(str::to_string),
        limit: get("limit").and_then(Value::as_u64).map(|n| n as usize),
        after: get("after").and_then(Value::as_str).map(str::to_string),
    }
}

/// A write's answer as the run sees it: the record, or the refusal.
type Wrote = std::result::Result<Value, Denied>;

impl Host {
    /// Answer the call `op` with `args`.
    pub fn call(&mut self, op: &str, args: &[Value]) -> Value {
        match self.answer(op, args) {
            Ok(v) => v,
            Err(e) => refused("internal", format!("{e:#}")),
        }
    }

    fn answer(&mut self, op: &str, args: &[Value]) -> Result<Value> {
        let ok = |v: Value| json!({ "ok": v });
        let m = self.manifest.clone();
        let coll = str_arg(args, 0).unwrap_or_default().to_string();
        Ok(match op {
            "list" => {
                let q = query_of(args.get(1));
                match docs::query(&self.store.conn(), &m, &coll, &self.caller, &q, false)? {
                    Ok(page) => ok(json!({
                        "docs": page.docs.iter().map(Doc::record).collect::<Vec<_>>(),
                        "after": page.after,
                    })),
                    Err(d) => denied(d),
                }
            }
            "get" => {
                let id = str_arg(args, 1).unwrap_or_default();
                match docs::get(&self.store.conn(), &m, &coll, &self.caller, id)? {
                    Ok(doc) => ok(doc.record()),
                    Err(d) => denied(d),
                }
            }
            "sum" => {
                let v = args.get(1);
                let get = |k: &str| {
                    v.and_then(|v| v.get(k))
                        .and_then(Value::as_str)
                        .map(str::to_string)
                };
                let s = Sum {
                    filter: get("where"),
                    group: get("group"),
                    of: get("of"),
                };
                match docs::sum(&self.store.conn(), &m, &coll, &self.caller, &s)? {
                    Ok(groups) => ok(json!(groups)),
                    Err(d) => denied(d),
                }
            }
            "search" => {
                let q = str_arg(args, 1).unwrap_or_default();
                let limit = args.get(2).and_then(Value::as_u64).unwrap_or(20) as usize;
                match docs::search(&self.store.conn(), &m, &coll, &self.caller, q, limit)? {
                    Ok(found) => ok(json!(found.iter().map(Doc::record).collect::<Vec<_>>())),
                    Err(d) => denied(d),
                }
            }
            "create" | "update" | "delete" => match self.write(op, &coll, args)? {
                Ok(v) => ok(v),
                Err(d) => denied(d),
            },
            "schedule" => self.schedule(args)?,
            "has" => ok(json!(str_arg(args, 0).is_some_and(|n| self.sources.has(n)))),
            "once_get" => {
                let key = str_arg(args, 0).unwrap_or_default();
                let held: Option<String> = self
                    .store
                    .conn()
                    .query_row(
                        "SELECT value FROM job_once WHERE app = ?1 AND key = ?2",
                        params![m.id, key],
                        |r| r.get(0),
                    )
                    .optional()?;
                match held {
                    Some(v) => ok(
                        json!({"value": serde_json::from_str::<Value>(&v).unwrap_or(Value::Null)}),
                    ),
                    None => ok(Value::Null),
                }
            }
            "once_put" => {
                let key = str_arg(args, 0).unwrap_or_default();
                let value = args.get(1).cloned().unwrap_or(Value::Null);
                self.store.conn().execute(
                    "INSERT OR REPLACE INTO job_once (app, key, value, at_ms) VALUES (?1, ?2, ?3, ?4)",
                    params![m.id, key, value.to_string(), now_ms()],
                )?;
                ok(Value::Null)
            }
            "time" => match args
                .first()
                .and_then(Value::as_f64)
                .and_then(|ms| spoken(ms as i64, &self.zone))
            {
                Some(t) => ok(json!(t)),
                None => refused("bad_request", "`ctx.time` takes a time in milliseconds"),
            },
            "now" => ok(json!(now_ms())),
            "log" => {
                if self.log.len() < LOG_LINES {
                    let line: String = str_arg(args, 0)
                        .unwrap_or_default()
                        .chars()
                        .take(LOG_LINE)
                        .collect();
                    self.log.push(line);
                }
                ok(Value::Null)
            }
            other => refused("bad_request", format!("ctx has no `{other}`")),
        })
    }

    /// A create, update or delete, committed and told to the open pages.
    fn write(&mut self, op: &str, coll: &str, args: &[Value]) -> Result<Wrote> {
        let m = self.manifest.clone();
        let now = now_ms();
        let mut conn = self.store.conn();
        let tx = conn.transaction()?;
        let (done, id, change) = match op {
            "create" => {
                let fields = args.get(1).cloned().unwrap_or(Value::Null);
                match docs::create(&tx, &m, coll, &self.caller, &fields, now)? {
                    Ok(doc) => (doc.record(), doc.id.clone(), Op::Created),
                    Err(d) => return Ok(Err(d)),
                }
            }
            "update" => {
                let id = str_arg(args, 1).unwrap_or_default().to_string();
                let patch = args.get(2).cloned().unwrap_or(Value::Null);
                let change = Change {
                    patch: &patch,
                    expect: args.get(3).and_then(Value::as_i64),
                };
                match docs::update(&tx, &m, coll, &self.caller, &id, change, now)? {
                    Ok(doc) => (doc.record(), id, Op::Updated),
                    Err(d) => return Ok(Err(d)),
                }
            }
            _ => {
                let id = str_arg(args, 1).unwrap_or_default().to_string();
                match docs::delete(&tx, &m, coll, &self.caller, &id, now)? {
                    Ok(()) => (json!({"deleted": id}), id, Op::Deleted),
                    Err(d) => return Ok(Err(d)),
                }
            }
        };
        let told = change_of(&tx, &m, coll, &id, change)?;
        tx.commit()?;
        self.changes.send(Feed::Changed(told));
        Ok(Ok(done))
    }

    /// `ctx.schedule(at, workflow, args)`: a job for later, acting for the
    /// run's account.
    fn schedule(&mut self, args: &[Value]) -> Result<Value> {
        let Some(at) = args.first().and_then(Value::as_f64) else {
            return Ok(refused(
                "bad_request",
                "`ctx.schedule` takes a time in milliseconds, or a Date",
            ));
        };
        let workflow = str_arg(args, 1).unwrap_or_default();
        if !is_name(workflow) || !self.sources.has(workflow) {
            return Ok(refused(
                "no_workflow",
                format!("there is no workflow `{workflow}`"),
            ));
        }
        let with = args.get(2).cloned().unwrap_or_else(|| json!({}));
        let id = insert(
            &self.store.conn(),
            &NewJob {
                app: &self.manifest.id,
                workflow,
                args: &with,
                source: "workflow",
                rule: None,
                catch_up: false,
                next_ms: at as i64,
                run_as: acting(self.caller.id()),
            },
            now_ms(),
        )?;
        self.scheduled = true;
        Ok(json!({ "ok": id }))
    }
}
