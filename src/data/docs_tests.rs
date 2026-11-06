//! The records against a real SQLite file.

use serde_json::json;

use super::*;
use crate::accounts::{Account, Status};
use crate::store::Store;

const APP: &str = r#"
id: home
name: Home
collections:
  recipes:
    fields:
      title:  { type: text, required: true }
      serves: { type: number, default: 2 }
      kind:   { type: text }
    search: [title]
  notes:
    scope: private
    fields:
      text: { type: text, required: true }
  chores:
    roles: { read: [everyone], write: [household] }
    fields:
      task: { type: text, required: true }
      done: { type: bool, default: false }
"#;

fn app() -> Manifest {
    Manifest::parse(APP, "home").unwrap()
}

fn caller(id: &str, groups: &[&str]) -> Caller {
    Caller {
        account: Account {
            id: id.into(),
            email: format!("{id}@example.com"),
            name: id.into(),
            picture: None,
            status: Status::Active,
            admin: false,
            created_ms: 0,
            seen_ms: 0,
        },
        groups: groups.iter().map(|g| g.to_string()).collect(),
        token: None,
    }
}

fn store() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(&dir.path().join("yenbot.db")).unwrap();
    (dir, store)
}

fn make(store: &Store, coll: &str, who: &Caller, data: Value) -> Answer<Doc> {
    let mut conn = store.conn();
    let tx = conn.transaction().unwrap();
    let done = create(&tx, &app(), coll, who, &data, 1_000).unwrap();
    tx.commit().unwrap();
    done
}

/// Merge `patch` into record `id` of `coll` as `who`, expecting version
/// `expect` when given.
fn change(
    tx: &Transaction<'_>,
    coll: &str,
    who: &Caller,
    id: &str,
    patch: Value,
    expect: Option<i64>,
) -> Answer<Doc> {
    let c = Change {
        patch: &patch,
        expect,
    };
    update(tx, &app(), coll, who, id, c, 2_000).unwrap()
}

fn shares(read: &[&str], edit: &[&str]) -> Shares {
    let names = |n: &[&str]| n.iter().map(|w| w.to_string()).collect();
    Shares {
        read: names(read),
        edit: names(edit),
    }
}

/// **A shared record is everyone's to read, a private one its owner's.**
#[test]
fn shared_records_are_everyones_and_private_ones_their_owners() {
    let (_d, s) = store();
    let (yen, mai) = (caller("yen", &[]), caller("mai", &[]));
    let read = |coll: &str, who: &Caller, id: &str| get(&s.conn(), &app(), coll, who, id).unwrap();
    let soup = make(&s, "recipes", &yen, json!({"title": "Phở"})).unwrap();
    assert_eq!(soup.data, json!({"title": "Phở", "serves": 2}));
    assert_eq!(read("recipes", &mai, &soup.id).unwrap().id, soup.id);
    let note = make(&s, "notes", &yen, json!({"text": "secret"})).unwrap();
    assert_eq!(read("notes", &mai, &note.id), Err(Denied::NotFound));
    assert!(read("notes", &yen, &note.id).is_ok());
    let refused = validate::Refused {
        field: "title".into(),
        reason: "must be text".into(),
    };
    assert_eq!(
        make(&s, "recipes", &yen, json!({"title": 1})),
        Err(Denied::Invalid(refused))
    );
    let nowhere = make(&s, "nope", &yen, json!({}));
    assert!(matches!(nowhere, Err(Denied::NoCollection(_))));
}

/// **A collection's roles hold**: everyone reads the chores, only the
/// household adds or ticks them.
#[test]
fn a_collections_roles_say_who_writes() {
    let (_d, s) = store();
    let (mum, kid) = (caller("mum", &["household"]), caller("kid", &[]));
    let chore = make(&s, "chores", &mum, json!({"task": "dishes"})).unwrap();
    let added = make(&s, "chores", &kid, json!({"task": "x"}));
    assert!(matches!(added, Err(Denied::Forbidden(_))));
    let mut conn = s.conn();
    let tx = conn.transaction().unwrap();
    let ticked = change(&tx, "chores", &kid, &chore.id, json!({"done": true}), None);
    assert!(matches!(ticked, Err(Denied::Forbidden(_))));
    let tick = |done: bool| {
        let patch = json!({ "done": done });
        change(&tx, "chores", &mum, &chore.id, patch, Some(1))
    };
    let done = tick(true).unwrap();
    assert_eq!((done.version, done.data["done"].clone()), (2, json!(true)));
    assert_eq!(tick(false), Err(Denied::Stale { held: 2 }));
}

/// **Queries filter, order and page**, and a cursor picks up where the last
/// page stopped.
#[test]
fn queries_filter_order_and_page() {
    let (_d, s) = store();
    let yen = caller("yen", &[]);
    for (title, serves) in [("a", 1), ("b", 4), ("c", 6), ("d", 4)] {
        let data = json!({"title": title, "serves": serves});
        make(&s, "recipes", &yen, data).unwrap();
    }
    let asked = |q: Query| query(&s.conn(), &app(), "recipes", &yen, &q, false).unwrap();
    let q = |filter: &str, order: &str, limit: usize, after: Option<String>| {
        asked(Query {
            filter: Some(filter.into()),
            order: (!order.is_empty()).then(|| order.into()),
            limit: Some(limit),
            after,
        })
        .unwrap()
    };
    let titles = |p: &Page| {
        p.docs
            .iter()
            .map(|d| d.data["title"].as_str().unwrap().to_string())
            .collect::<Vec<_>>()
    };
    // Descending breaks ties newest first, as it orders everything else.
    let most = q("serves >= 4", "-serves", 10, None);
    assert_eq!(titles(&most), ["c", "d", "b"]);
    let first = q("", "", 3, None);
    assert_eq!(titles(&first), ["a", "b", "c"]);
    let rest = q("", "", 3, first.after.clone());
    assert_eq!((titles(&rest), rest.after), (vec!["d".to_string()], None));
    let by_serves = q("", "serves", 2, None);
    assert_eq!(titles(&by_serves), ["a", "b"]);
    assert_eq!(titles(&q("", "serves", 2, by_serves.after)), ["d", "c"]);
    let nonsense = asked(Query {
        filter: Some("nope = 1".into()),
        ..Query::default()
    });
    assert!(matches!(nonsense, Err(Denied::Bad(_))));
}

#[test]
fn sums_count_and_total_by_group() {
    let (_d, s) = store();
    let yen = caller("yen", &[]);
    for (kind, serves) in [("soup", 2), ("soup", 4), ("cake", 8)] {
        let data = json!({"title": "x", "kind": kind, "serves": serves});
        make(&s, "recipes", &yen, data).unwrap();
    }
    let by_kind = Sum {
        group: Some("kind".into()),
        of: Some("serves".into()),
        ..Sum::default()
    };
    let sums = sum(&s.conn(), &app(), "recipes", &yen, &by_kind)
        .unwrap()
        .unwrap();
    assert_eq!(
        sums,
        vec![
            json!({"kind": "cake", "count": 1, "sum": 8, "avg": 8.0, "min": 8, "max": 8}),
            json!({"kind": "soup", "count": 2, "sum": 6, "avg": 3.0, "min": 2, "max": 4}),
        ]
    );
}

/// **"pho" finds "phở", "banh da" finds "Bánh đa"**, and an update is
/// searched as it now reads.
#[test]
fn search_folds_diacritics() {
    let (_d, s) = store();
    let yen = caller("yen", &[]);
    let pho = make(&s, "recipes", &yen, json!({"title": "Phở bò"})).unwrap();
    make(&s, "recipes", &yen, json!({"title": "Bánh đa cua"})).unwrap();
    let found = |q: &str| {
        search(&s.conn(), &app(), "recipes", &yen, q, 10)
            .unwrap()
            .unwrap()
            .iter()
            .map(|d| d.data["title"].as_str().unwrap().to_string())
            .collect::<Vec<_>>()
    };
    assert_eq!(found("pho"), ["Phở bò"]);
    assert_eq!(found("banh da"), ["Bánh đa cua"]);
    assert_eq!(found("\"); DROP"), Vec::<String>::new());
    let mut conn = s.conn();
    let tx = conn.transaction().unwrap();
    let bun = json!({"title": "Bún chả"});
    change(&tx, "recipes", &yen, &pho.id, bun, None).unwrap();
    tx.commit().unwrap();
    drop(conn);
    assert_eq!(found("pho"), Vec::<String>::new());
    assert_eq!(found("bun"), ["Bún chả"]);
}

/// **A batch is all or nothing**, and the refusal names the write.
#[test]
fn a_batch_is_all_or_nothing() {
    let (_d, s) = store();
    let yen = caller("yen", &[]);
    let writes: Vec<Write> = serde_json::from_value(json!([
        {"op": "create", "coll": "recipes", "data": {"title": "a"}},
        {"op": "create", "coll": "recipes", "data": {}},
    ]))
    .unwrap();
    let mut conn = s.conn();
    let tx = conn.transaction().unwrap();
    let (at, why) = batch(&tx, &app(), &yen, &writes, 1).unwrap().unwrap_err();
    assert_eq!(at, 1);
    assert!(matches!(why, Denied::Invalid(_)));
    drop(tx);
    drop(conn);
    let none = query(&s.conn(), &app(), "recipes", &yen, &Query::default(), false)
        .unwrap()
        .unwrap();
    assert!(none.docs.is_empty(), "undone, the first with the second");
}

/// **A private record shared with a group is theirs to read; shared to
/// edit, theirs to change.**
#[test]
fn a_shared_record_is_read_or_edited_by_whom_it_names() {
    let (_d, s) = store();
    let (yen, gran) = (caller("yen", &[]), caller("gran", &["grandparents"]));
    let list = make(&s, "notes", &yen, json!({"text": "milk"})).unwrap();
    let mut conn = s.conn();
    let tx = conn.transaction().unwrap();
    let to = |s: Shares| share(&tx, &app(), "notes", &yen, &list.id, &s).unwrap();
    to(shares(&["group:grandparents"], &[])).unwrap();
    assert!(get(&tx, &app(), "notes", &gran, &list.id).unwrap().is_ok());
    let eggs = || change(&tx, "notes", &gran, &list.id, json!({"text": "eggs"}), None);
    assert!(matches!(eggs(), Err(Denied::Forbidden(_))));
    to(shares(&[], &["account:gran"])).unwrap();
    assert!(eggs().is_ok());
    assert!(matches!(
        share(&tx, &app(), "notes", &gran, &list.id, &Shares::default()).unwrap(),
        Err(Denied::Forbidden(_))
    ));
}
