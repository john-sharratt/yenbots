use std::collections::BTreeMap;
use std::path::Path;

use jiff::tz::TimeZone;
use serde_json::json;

use super::*;
use crate::ai::fake::FakeZen;
use crate::config::Config;
use crate::data::changes::Changes;
use crate::files::Files;
use crate::notify::push::Pusher;
use crate::store::Store;

/// Recipes at shape 3: `title` and `serves`, where shape 1 had `name` and
/// shape 2 `title`.
const APP: &str = "id: home\nname: Home\ncollections:\n  recipes:\n    version: 3\n    fields:\n      \
                   title: { type: text, required: true }\n      serves: { type: number }\n    search: [title]\n";

fn env() -> Env {
    Env {
        store: Arc::new(Store::in_memory().unwrap()),
        changes: Changes::new(),
        zone: TimeZone::UTC,
        cfg: Config::from_yaml(
            "data_dir: d\nlive_dir: l\nadmins: [a@b.c]\ntime_zone: UTC\nbus: {url: u, key_file: k}\n",
            Path::new("."),
        )
        .unwrap(),
        pusher: Pusher::channel().0,
        zen: Arc::new(FakeZen::default()),
        files: Files::new("files"),
    }
}

fn sources(files: &[(&str, &str)]) -> Sources {
    let files: BTreeMap<String, String> = files
        .iter()
        .map(|(name, text)| (format!("apps/home/workflows/{name}.js"), text.to_string()))
        .collect();
    Sources {
        dir: "apps/home/workflows/".into(),
        files: Arc::new(files),
    }
}

fn put(env: &Env, id: &str, data: Value, shape: u32) {
    env.store
        .conn()
        .execute(
            "INSERT INTO docs (app, coll, id, owner, data, version, created_ms, updated_ms, shape) \
             VALUES ('home', 'recipes', ?1, '1', ?2, 1, 0, 0, ?3)",
            params![id, data.to_string(), shape],
        )
        .unwrap();
}

fn rows(env: &Env) -> Vec<(String, Value, u32, i64)> {
    let conn = env.store.conn();
    let mut stmt = conn
        .prepare("SELECT id, data, shape, version FROM docs ORDER BY id")
        .unwrap();
    stmt.query_map([], |r| {
        let data: String = r.get(1)?;
        Ok((
            r.get(0)?,
            serde_json::from_str(&data).unwrap(),
            r.get(2)?,
            r.get(3)?,
        ))
    })
    .unwrap()
    .collect::<Result<_, _>>()
    .unwrap()
}

const TWO: &str =
    "export default async function m(ctx, { record }) { return { title: record.name }; }";
const THREE: &str =
    "export default async function m(ctx, { record }) { return { ...record, serves: 2 }; }";

/// **Each record is taken up every step to the manifest's version, held to
/// its fields, searchable again, its progress kept.**
#[test]
fn records_are_taken_up_each_step() {
    let env = env();
    let m = Arc::new(Manifest::parse(APP, "home").unwrap());
    put(&env, "r1", json!({"name": "Phở"}), 1);
    put(&env, "r2", json!({"title": "Bún chả"}), 2);
    put(&env, "r3", json!({"title": "Cơm tấm", "serves": 4}), 3);
    assert_eq!(behind(&env.store.conn(), &m).unwrap(), ["recipes"]);
    let src = sources(&[("migrate-recipes-2", TWO), ("migrate-recipes-3", THREE)]);
    assert_eq!(migrate(&env, &m, &src, "recipes", 50).unwrap(), 2);
    assert_eq!(
        rows(&env),
        [
            ("r1".to_string(), json!({"title": "Phở", "serves": 2}), 3, 3),
            (
                "r2".to_string(),
                json!({"title": "Bún chả", "serves": 2}),
                3,
                2
            ),
            (
                "r3".to_string(),
                json!({"title": "Cơm tấm", "serves": 4}),
                3,
                1
            ),
        ]
    );
    assert!(behind(&env.store.conn(), &m).unwrap().is_empty());
    let found: String = env
        .store
        .conn()
        .query_row(
            "SELECT id FROM docs_fts WHERE docs_fts MATCH 'pho'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(found, "r1");
    let kept: (u32, u64, u64, String) = env
        .store
        .conn()
        .query_row("SELECT shape, done, total, state FROM reshapes WHERE app = 'home' AND coll = 'recipes'", [], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })
        .unwrap();
    assert_eq!(kept, (3, 2, 2, "done".to_string()));
}

/// **A record written while its batch was being migrated keeps that
/// write**: it is read again and taken up from what it then holds.
#[test]
fn a_write_during_a_migration_is_kept() {
    let env = env();
    let m = Arc::new(Manifest::parse(APP, "home").unwrap());
    put(&env, "r1", json!({"title": "Phở"}), 2);
    // The first time it is handed the record, the migration writes to it —
    // as a person would, mid-batch — before answering.
    let busy = "export default async function m(ctx, { record }) {\n\
                  if (!record.title.endsWith('*')) {\n\
                    await ctx.data.collection('recipes').update(record.id, { title: record.title + '*' });\n\
                  }\n\
                  return { title: record.title, serves: 2 };\n\
                }";
    migrate(
        &env,
        &m,
        &sources(&[("migrate-recipes-3", busy)]),
        "recipes",
        50,
    )
    .unwrap();
    // The write was held to the new fields, so the record is at the new
    // shape as it was written, and the migration's answer is not laid over it.
    assert_eq!(rows(&env)[0].1, json!({"title": "Phở*"}));
    assert_eq!(rows(&env)[0].2, 3);
}

/// **A migration that returns a record its fields refuse stops, saying
/// which and why, and leaves that batch as it was**; one that is not there
/// is said so.
#[test]
fn a_migration_refused_stops_and_says_why() {
    let env = env();
    let m = Arc::new(Manifest::parse(APP, "home").unwrap());
    put(&env, "r1", json!({"title": "Phở"}), 2);
    let bad =
        "export default async function m(ctx, { record }) { return { ...record, serves: 'two' }; }";
    let e = migrate(
        &env,
        &m,
        &sources(&[("migrate-recipes-3", bad)]),
        "recipes",
        50,
    )
    .unwrap_err();
    assert_eq!(
        e.to_string(),
        "apps/home/workflows/migrate-recipes-3.js for record r1: serves must be a number"
    );
    assert_eq!(rows(&env)[0].2, 2, "still at its shape");
    let (state, error): (String, String) = env
        .store
        .conn()
        .query_row("SELECT state, error FROM reshapes", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap();
    assert_eq!((state.as_str(), error), ("failed", e.to_string()));

    let e = migrate(&env, &m, &sources(&[]), "recipes", 50).unwrap_err();
    assert_eq!(
        e.to_string(),
        "there is no workflows/migrate-recipes-3.js to take recipes from shape 2 to 3"
    );
}
