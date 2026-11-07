//! A collection's records taken up to its manifest's version (§7.5): each
//! record at shape `n - 1` handed to `workflows/migrate-<coll>-<n>.js`,
//! which returns its fields at shape `n`, a batch at a time — one engine
//! run and one transaction a batch, its progress kept in `reshapes`. A
//! record taken to the manifest's own version is held to its fields, as any
//! write is; one refused stops the migration, with the record and why.
//!
//! Every record keeps its shape, so a migration cut short carries on where
//! it stopped, and one written meanwhile at an older shape is taken up too.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection};
use serde_json::{json, Map, Value};

use crate::accounts::caller::Caller;
use crate::apps::manifest::{Manifest, RESERVED};
use crate::data::docs::Doc;
use crate::data::search::{body_of, index, unindex};
use crate::data::validate;
use crate::workflows::engine::{evaluate, Driver};
use crate::workflows::loader::Sources;
use crate::workflows::run::{host_for, run_info, Env};

/// Records migrated in one run.
pub const BATCH: usize = 100;

/// The workflow that takes `coll`'s records to shape `n`.
pub fn migration_of(coll: &str, n: u32) -> String {
    format!("migrate-{coll}-{n}")
}

/// The driver that hands each record to migration `path` in turn.
fn driver_of(path: &str) -> String {
    format!(
        "import migrate from {};\nimport {{ ctx }} from \"yen:ctx\";\n\
         globalThis.__yen_done = (async () => {{\n  \
           const out = [];\n  \
           for (const record of JSON.parse(globalThis.__yen_args)) out.push(await migrate(ctx, {{ record }}));\n  \
           return JSON.stringify(out);\n\
         }})();\n",
        json!(path)
    )
}

/// The lowest shape among `coll`'s records below `version`, and how many
/// are below it.
fn below(conn: &Connection, app: &str, coll: &str, version: u32) -> Result<(Option<u32>, u64)> {
    Ok(conn.query_row(
        "SELECT MIN(shape), COUNT(*) FROM docs WHERE app = ?1 AND coll = ?2 AND shape < ?3",
        params![app, coll, version],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?)
}

/// The collections of `m` holding a record below its version.
pub fn behind(conn: &Connection, m: &Manifest) -> Result<Vec<String>> {
    let mut out = Vec::new();
    for (name, c) in &m.collections {
        if below(conn, &m.id, name, c.version)?.0.is_some() {
            out.push(name.clone());
        }
    }
    Ok(out)
}

/// How far a migration has come.
struct Progress {
    /// The shape it takes the records to.
    shape: u32,
    done: u64,
    total: u64,
    state: &'static str,
    error: Option<String>,
}

fn progress(conn: &Connection, m: &Manifest, coll: &str, p: &Progress, now: i64) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO reshapes (app, coll, shape, done, total, state, error, updated_ms) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![m.id, coll, p.shape, p.done, p.total, p.state, p.error, now],
    )?;
    Ok(())
}

/// The fields a migration returned for a record: its own names dropped.
fn fields_of(out: &Value) -> Result<Value> {
    let Value::Object(given) = out else {
        bail!("returned {out}, not the record's fields");
    };
    let kept: Map<String, Value> = given
        .iter()
        .filter(|(k, _)| !RESERVED.contains(&k.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    Ok(Value::Object(kept))
}

/// Take `coll` of app `m` up to its version with the migrations in
/// `sources`, at `now`; answers how many records were migrated.
pub fn migrate(
    env: &Env,
    m: &Arc<Manifest>,
    sources: &Sources,
    coll: &str,
    now: i64,
) -> Result<u64> {
    let c = m
        .collections
        .get(coll)
        .ok_or_else(|| anyhow!("no collection `{coll}`"))?;
    let total = below(&env.store.conn(), &m.id, coll, c.version)?.1;
    let mut done = 0;
    let report = |done: u64, state: &'static str, error: Option<String>| Progress {
        shape: c.version,
        done,
        total,
        state,
        error,
    };
    let fail = |done: u64, why: String| -> Result<u64> {
        progress(
            &env.store.conn(),
            m,
            coll,
            &report(done, "failed", Some(why.clone())),
            now,
        )?;
        bail!("{why}")
    };
    loop {
        let (Some(at), _) = below(&env.store.conn(), &m.id, coll, c.version)? else {
            break;
        };
        let step = at + 1;
        let name = migration_of(coll, step);
        if !sources.has(&name) {
            return fail(
                done,
                format!("there is no workflows/{name}.js to take {coll} from shape {at} to {step}"),
            );
        }
        let batch: Vec<Doc> = {
            let conn = env.store.conn();
            let mut stmt = conn.prepare(
                "SELECT id, owner, data, version, created_ms, updated_ms FROM docs \
                 WHERE app = ?1 AND coll = ?2 AND shape = ?3 ORDER BY id LIMIT ?4",
            )?;
            let rows = stmt.query_map(params![m.id, coll, at, BATCH as i64], |r| {
                let data: String = r.get(2)?;
                Ok(Doc {
                    id: r.get(0)?,
                    owner: r.get(1)?,
                    data: serde_json::from_str(&data).unwrap_or(Value::Null),
                    version: r.get(3)?,
                    created_ms: r.get(4)?,
                    updated_ms: r.get(5)?,
                })
            })?;
            rows.collect::<Result<_, _>>()?
        };
        let path = sources.path_of(&name);
        let caller = Caller::app(&m.id, &m.name);
        let driver = Driver {
            source: driver_of(&path),
            path: &path,
            run: run_info(m, sources, &caller, None),
            args: Value::Array(batch.iter().map(Doc::record).collect()),
        };
        let deadline = Duration::from_millis(m.deadline_ms().unwrap_or(60_000) as u64);
        let host = host_for(env, m.clone(), sources.clone(), caller);
        let (_, answer) = evaluate(host, sources, driver, deadline);
        let out = match answer {
            Ok(Value::Array(out)) if out.len() == batch.len() => out,
            Ok(other) => {
                return fail(
                    done,
                    format!("{path} answered {other}, not one record for each"),
                )
            }
            Err(f) => return fail(done, f.to_string()),
        };
        let last = step == c.version;
        let mut conn = env.store.conn();
        let tx = conn.transaction()?;
        for (doc, out) in batch.iter().zip(&out) {
            let fields = match fields_of(out) {
                Ok(f) => f,
                Err(e) => {
                    drop(tx);
                    drop(conn);
                    return fail(done, format!("{path} for record {}: {e}", doc.id));
                }
            };
            let fields = if last {
                match validate::create(&c.fields, &fields) {
                    Ok(f) => f,
                    Err(r) => {
                        drop(tx);
                        drop(conn);
                        return fail(
                            done,
                            format!("{path} for record {}: {} {}", doc.id, r.field, r.reason),
                        );
                    }
                }
            } else {
                fields
            };
            tx.execute(
                "UPDATE docs SET data = ?4, shape = ?5, version = version + 1, updated_ms = ?6 \
                 WHERE app = ?1 AND coll = ?2 AND id = ?3",
                params![m.id, coll, doc.id, fields.to_string(), step, now],
            )?;
            if last {
                unindex(&tx, &m.id, coll, &doc.id)?;
                index(&tx, &m.id, coll, &doc.id, &body_of(&c.search, &fields))?;
            }
        }
        tx.commit()?;
        drop(conn);
        if last {
            done += batch.len() as u64;
        }
        progress(
            &env.store.conn(),
            m,
            coll,
            &report(done, "running", None),
            now,
        )?;
    }
    progress(&env.store.conn(), m, coll, &report(done, "done", None), now)?;
    Ok(done)
}

#[cfg(test)]
#[path = "migrate_tests.rs"]
mod tests;
