use std::path::Path;
use std::sync::Arc;

use super::*;
use crate::apps::Apps;
use crate::config::Config;
use crate::store::Store;

fn write_app(live: &Path, id: &str, extra: &str) {
    let dir = live.join("apps").join(id);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("app.yaml"),
        format!("id: {id}\nname: {id}\ncollections:\n  items:\n    fields:\n      text: {{ type: text }}\n{extra}"),
    )
    .unwrap();
}

fn site(dir: &Path) -> Arc<App> {
    let live = dir.join("live");
    write_app(&live, "chores", "");
    let cfg = Config::from_yaml(
        "data_dir: data\nlive_dir: live\nadmins: [yen@example.com]\ntime_zone: UTC\n\
         bus: {url: u, key_file: k}\n",
        dir,
    )
    .unwrap();
    let store = Arc::new(Store::in_memory().unwrap());
    store
        .conn()
        .execute_batch(
            "INSERT INTO docs (app, coll, id, owner, data, version, created_ms, updated_ms) VALUES \
               ('chores', 'items', 'c1', '1', '{}', 1, 0, 0), \
               ('recipe', 'items', 'r1', '1', '{}', 1, 0, 0);",
        )
        .unwrap();
    let apps = Apps::open(&live, store.clone());
    assert!(apps.get("chores").is_some(), "{:?}", apps.checked("chores"));
    Arc::new(App::for_tests(cfg, store, apps))
}

/// **An app whose folder goes is archived and not served; its folder coming
/// back restores it; a rename moves the old id's records to the new, once.**
#[test]
fn the_store_follows_the_folders() {
    let dir = tempfile::tempdir().unwrap();
    let app = site(dir.path());
    let live = dir.path().join("live");

    let first = settle(&app, 10).unwrap();
    assert_eq!(first.archived, ["recipe"], "data and no folder");
    assert_eq!(settle(&app, 11).unwrap(), Settled::default(), "once");

    std::fs::remove_dir_all(live.join("apps").join("chores")).unwrap();
    app.apps.reload();
    assert_eq!(settle(&app, 12).unwrap().archived, ["chores"]);
    assert!(app.apps.get("chores").is_none());

    write_app(&live, "chores", "");
    app.apps.reload();
    assert!(
        app.apps.get("chores").is_none(),
        "archived, though its folder is back"
    );
    assert_eq!(settle(&app, 13).unwrap().restored, ["chores"]);
    assert!(app.apps.get("chores").is_some());

    write_app(&live, "recipes", "renamed_from: recipe\n");
    app.apps.reload();
    let moved = settle(&app, 14).unwrap();
    assert_eq!(
        moved.renamed,
        [("recipe".to_string(), "recipes".to_string())]
    );
    let held: i64 = app
        .store
        .conn()
        .query_row("SELECT COUNT(*) FROM docs WHERE app = 'recipes'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(held, 1);
    assert!(
        archive::ids(&app.store.conn()).unwrap().is_empty(),
        "the old id's archive went with it"
    );
    assert_eq!(settle(&app, 15).unwrap(), Settled::default());
}
