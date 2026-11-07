use std::path::Path;

use serde_json::{json, Value};

use super::*;
use crate::apps::Apps;
use crate::config::Config;
use crate::jobs::table::{get, insert, list, runs, NewJob};
use crate::logs::Search;
use crate::store::Store;

const MANIFEST: &str =
    "id: home\nname: Home\ncollections:\n  reminders:\n    scope: private\n    fields:\n      \
                        title: { type: text }\n      due_ms: { type: time }\n\
                        schedules:\n  - from: reminders\n    at: due_ms\n    run: remind\n";

const REMIND: &str = "export default async function remind(ctx, { record }) {\n\
                        ctx.log('remind', record.title, 'for', ctx.account);\n\
                        return record.title;\n\
                      }";

const BROKEN: &str =
    "export default async function broken(ctx) {\n  throw new Error('no network');\n}";

/// A site whose live checkout holds the app `home`, its store in memory.
fn site(dir: &Path) -> Arc<App> {
    let live = dir.join("live");
    let wf = live.join("apps/home/workflows");
    std::fs::create_dir_all(&wf).unwrap();
    std::fs::write(live.join("apps/home/app.yaml"), MANIFEST).unwrap();
    std::fs::write(wf.join("remind.js"), REMIND).unwrap();
    std::fs::write(wf.join("broken.js"), BROKEN).unwrap();
    let cfg = Config::from_yaml(
        "data_dir: data\nlive_dir: live\nadmins: [yen@example.com]\ntime_zone: Australia/Sydney\n\
         bus: {url: u, key_file: k}\n",
        dir,
    )
    .unwrap();
    let store = Arc::new(Store::in_memory().unwrap());
    let apps = Apps::open(&live, store.clone());
    assert!(apps.get("home").is_some(), "{:?}", apps.checked("home"));
    Arc::new(App::for_tests(cfg, store, apps))
}

fn job<'a>(
    workflow: &'a str,
    source: &'a str,
    rule: Option<&'a str>,
    next_ms: i64,
    args: &'a Value,
) -> NewJob<'a> {
    NewJob {
        app: "home",
        workflow,
        args,
        source,
        rule,
        catch_up: false,
        next_ms,
        run_as: None,
    }
}

const HOUR: i64 = 3_600_000;

/// **A job missed by over an hour is passed over — a repeating one moves
/// to its next time — unless its schedule catches up; one missed by less
/// runs, and one left running is due again.**
#[test]
fn what_fell_due_while_down_is_settled() {
    let dir = tempfile::tempdir().unwrap();
    let app = site(dir.path());
    let now = 1_793_980_800_000;
    let none = json!({});
    let conn = app.store.conn();
    let old = insert(
        &conn,
        &job(
            "remind",
            "schedule:0",
            Some("daily 07:30"),
            now - 2 * HOUR,
            &none,
        ),
        0,
    )
    .unwrap();
    let mut caught = job(
        "remind",
        "schedule:1",
        Some("daily 07:30"),
        now - 9 * HOUR,
        &none,
    );
    caught.catch_up = true;
    let caught = insert(&conn, &caught, 0).unwrap();
    let recent = insert(
        &conn,
        &job("remind", "workflow", None, now - HOUR / 2, &none),
        0,
    )
    .unwrap();
    let once = insert(
        &conn,
        &job("remind", "workflow", None, now - 3 * HOUR, &none),
        0,
    )
    .unwrap();
    conn.execute("UPDATE jobs SET state = 'running' WHERE id = ?1", [&recent])
        .unwrap();
    drop(conn);

    let mut passed = catch_up(&app, now).unwrap();
    passed.sort();
    let mut expected = vec![old.clone(), once.clone()];
    expected.sort();
    assert_eq!(passed, expected);
    let conn = app.store.conn();
    let o = get(&conn, &old).unwrap().unwrap();
    assert_eq!(
        (o.state, o.next_ms),
        (State::Waiting, Some(now + 4 * HOUR + HOUR / 2)),
        "07:30 in Sydney"
    );
    let missed = runs(&conn, &old, 5).unwrap();
    assert!(!missed[0].ok);
    assert_eq!(
        missed[0].error.as_deref(),
        Some("missed: the site was down when it fell due, Saturday 7 November, 1:00 am, and came back over an hour later")
    );
    assert_eq!(get(&conn, &once).unwrap().unwrap().state, State::Failed);
    assert_eq!(
        get(&conn, &caught).unwrap().unwrap().next_ms,
        Some(now - 9 * HOUR),
        "caught up: runs"
    );
    let r = get(&conn, &recent).unwrap().unwrap();
    assert_eq!(
        (r.state, r.next_ms),
        (State::Waiting, Some(now - HOUR / 2)),
        "requeued, and runs"
    );
}

/// **A reminder's job runs its workflow with the record as it is now, as
/// its owner, and is done — its run kept and written to the app's log.**
#[test]
fn a_records_job_runs_with_its_record() {
    let dir = tempfile::tempdir().unwrap();
    let app = site(dir.path());
    let mut conn = app.store.conn();
    conn.execute(
        "INSERT INTO accounts (id, provider, email, name, status, created_ms, seen_ms) \
         VALUES ('1', 'google', 'yen@example.com', 'Yen', 'active', 0, 0)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO docs (app, coll, id, owner, data, version, created_ms, updated_ms) \
         VALUES ('home', 'reminders', 'r1', '1', '{\"title\": \"Call Mai\", \"due_ms\": 5}', 1, 0, 0)",
        [],
    )
    .unwrap();
    let args = json!({"coll": "reminders", "id": "r1"});
    let mut j = job("remind", "record:0:reminders/r1", None, 5, &args);
    j.run_as = Some("1");
    let id = insert(&conn, &j, 0).unwrap();
    let mut claimed = table::claim(&mut conn, &["home".into()], 10, 4).unwrap();
    drop(conn);
    run_job(&app, &claimed.remove(0));

    let conn = app.store.conn();
    let done = get(&conn, &id).unwrap().unwrap();
    assert_eq!((done.state, done.next_ms), (State::Done, None));
    let kept = runs(&conn, &id, 5).unwrap();
    assert!(kept[0].ok, "{:?}", kept[0].error);
    assert_eq!(kept[0].log, ["remind Call Mai for 1"]);
    drop(conn);
    let lines = app
        .logs
        .search(&Search {
            source: Some("yenbot/home".into()),
            limit: 10,
            ..Search::default()
        })
        .unwrap()
        .lines;
    assert_eq!(lines.len(), 1);
    assert!(
        lines[0].contains(&format!(
            "INFO yenbot/home workflow: workflow remind (job {id}) ran in "
        )),
        "{}",
        lines[0]
    );
    assert!(
        lines[0].ends_with("\nremind Call Mai for 1"),
        "{}",
        lines[0]
    );
}

/// **A run that fails is tried again later, its error kept and logged.**
#[test]
fn a_failed_run_is_tried_again() {
    let dir = tempfile::tempdir().unwrap();
    let app = site(dir.path());
    let none = json!({});
    let mut conn = app.store.conn();
    let id = insert(&conn, &job("broken", "workflow", None, 5, &none), 0).unwrap();
    let mut claimed = table::claim(&mut conn, &["home".into()], 10, 4).unwrap();
    drop(conn);
    run_job(&app, &claimed.remove(0));
    let conn = app.store.conn();
    let j = get(&conn, &id).unwrap().unwrap();
    assert_eq!((j.state, j.attempts), (State::Waiting, 1));
    let kept = runs(&conn, &id, 5).unwrap();
    assert_eq!(
        kept[0].error.as_deref(),
        Some("apps/home/workflows/broken.js:2 no network")
    );
}

/// **A record's job whose record has gone is done, with nothing run.**
#[test]
fn a_job_whose_record_is_gone_is_done() {
    let dir = tempfile::tempdir().unwrap();
    let app = site(dir.path());
    let args = json!({"coll": "reminders", "id": "gone"});
    let mut conn = app.store.conn();
    let id = insert(
        &conn,
        &job("remind", "record:0:reminders/gone", None, 5, &args),
        0,
    )
    .unwrap();
    let mut claimed = table::claim(&mut conn, &["home".into()], 10, 4).unwrap();
    drop(conn);
    run_job(&app, &claimed.remove(0));
    let conn = app.store.conn();
    assert_eq!(get(&conn, &id).unwrap().unwrap().state, State::Done);
    assert!(runs(&conn, &id, 5).unwrap().is_empty());
}

/// **The jobs of every served app and the platform's are made at start.**
#[test]
fn every_apps_jobs_are_made() {
    let dir = tempfile::tempdir().unwrap();
    let app = site(dir.path());
    app.store
        .conn()
        .execute(
            "INSERT INTO docs (app, coll, id, owner, data, version, created_ms, updated_ms) \
             VALUES ('home', 'reminders', 'r1', '1', '{\"due_ms\": 9000000000000}', 1, 0, 0)",
            [],
        )
        .unwrap();
    sync_all(&app, 0).unwrap();
    let all = list(&app.store.conn(), None, None).unwrap();
    let mut made: Vec<(&str, &str)> = all
        .iter()
        .map(|j| (j.app.as_str(), j.workflow.as_str()))
        .collect();
    made.sort();
    assert_eq!(
        made,
        [
            ("_platform", "backup"),
            ("_platform", "tidy"),
            ("home", "remind")
        ]
    );
}
