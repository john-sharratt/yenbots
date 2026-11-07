use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;

use super::*;
use crate::apps::Apps;
use crate::config::Config;
use crate::store::Store;

/// **The worker takes an app waiting for its migration up, and says it is
/// ready** — within seconds: a store held across the migration would never
/// answer.
#[test]
fn the_worker_migrates_a_waiting_app() {
    let dir = tempfile::tempdir().unwrap();
    let live = dir.path().join("live");
    let notes = live.join("apps/notes");
    std::fs::create_dir_all(notes.join("workflows")).unwrap();
    std::fs::write(
        notes.join("app.yaml"),
        "id: notes\nname: Notes\ncollections:\n  notes:\n    version: 2\n    fields:\n      text: { type: text }\n",
    )
    .unwrap();
    std::fs::write(
        notes.join("workflows/migrate-notes-2.js"),
        "export default async function m(ctx, { record }) { return { text: record.text + '!' }; }",
    )
    .unwrap();
    let store = Arc::new(Store::in_memory().unwrap());
    store
        .conn()
        .execute(
            "INSERT INTO docs (app, coll, id, owner, data, version, created_ms, updated_ms) \
             VALUES ('notes', 'notes', 'n1', '1', '{\"text\": \"hi\"}', 1, 0, 0)",
            [],
        )
        .unwrap();
    let apps = Apps::open(&live, store.clone());
    assert_eq!(apps.pending().len(), 1, "{:?}", apps.checked("notes"));
    let cfg = Config::from_yaml(
        "data_dir: data\nlive_dir: live\nadmins: [a@b.c]\ntime_zone: UTC\nbus: {url: u, key_file: k}\n",
        dir.path(),
    )
    .unwrap();
    let app = Arc::new(App::for_tests(cfg, store.clone(), apps));

    let (tx, rx) = channel();
    let worker = app.clone();
    thread::spawn(move || {
        let failed = Mutex::new(BTreeSet::new());
        let _ = tx.send(work(&worker, &failed));
    });
    let ready = rx
        .recv_timeout(Duration::from_secs(20))
        .expect("the worker answers");
    assert!(ready);
    let text: String = store
        .conn()
        .query_row(
            "SELECT json_extract(data, '$.text') FROM docs WHERE id = 'n1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(text, "hi!");
    app.apps.reload();
    assert_eq!(
        app.apps.get("notes").unwrap().collections["notes"].version,
        2
    );
}
