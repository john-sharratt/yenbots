use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;

use serde_json::json;

use super::*;
use crate::accounts::{Account, Status};
use crate::data::changes::Feed;
use crate::jobs::table::list;

const APP: &str =
    "id: home\nname: Home\ncollections:\n  notes:\n    scope: private\n    fields:\n      \
                   text: { type: text, required: true }\n      due_ms: { type: time }\n";

fn env() -> Env {
    Env {
        store: Arc::new(Store::in_memory().unwrap()),
        changes: Changes::new(),
        zone: TimeZone::get("Australia/Sydney").unwrap(),
        cfg: Config::from_yaml(
            "data_dir: d\nlive_dir: l\nadmins: [yen@example.com]\ntime_zone: UTC\nbus: {url: u, key_file: k}\n",
            Path::new("."),
        )
        .unwrap(),
        pusher: Pusher::channel().0,
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

fn manifest(extra: &str) -> Arc<Manifest> {
    Arc::new(Manifest::parse(&format!("{APP}{extra}"), "home").unwrap())
}

fn yen() -> Caller {
    Caller {
        account: Account {
            id: "1".into(),
            email: "yen@example.com".into(),
            name: "Yen".into(),
            picture: None,
            status: Status::Active,
            admin: true,
            created_ms: 0,
            seen_ms: 0,
        },
        groups: Vec::new(),
        token: None,
    }
}

fn ask(workflow: &str, args: Value, caller: Caller) -> Ask {
    Ask {
        workflow: workflow.into(),
        args,
        caller,
        job: None,
    }
}

fn go(env: &Env, files: &[(&str, &str)], workflow: &str, args: Value, caller: Caller) -> Outcome {
    run(
        env,
        manifest(""),
        sources(files),
        ask(workflow, args, caller),
    )
}

/// **A workflow writes and reads the app's records, logs, and answers**;
/// `ctx` says whose run it is and tells time in Yen's zone.
#[test]
fn a_workflow_writes_reads_and_answers() {
    let env = env();
    let mut feed = env.changes.subscribe();
    let src = "export default async function add(ctx, { text }) {\n\
                 const notes = ctx.data.collection('notes');\n\
                 const made = await notes.create({ text, due_ms: 1793980800000 });\n\
                 await notes.update(made.id, { text: text + '!' });\n\
                 const page = await notes.list({ order: 'text' });\n\
                 ctx.log('made', made.id.length, { n: page.docs.length });\n\
                 return { text: page.docs[0].text, owner: page.docs[0].owner, app: ctx.app,\n\
                          account: ctx.account, job: ctx.job, when: ctx.time(made.due_ms) };\n\
               }";
    let out = go(&env, &[("add", src)], "add", json!({"text": "Phở"}), yen());
    let answer = out.answer.unwrap();
    assert_eq!(
        answer,
        json!({"text": "Phở!", "owner": "1", "app": "home", "account": "1", "job": null,
               "when": "Saturday 7 November, 3:00 am"})
    );
    assert_eq!(out.log, ["made 20 {\"n\":1}"]);
    assert!(!out.scheduled);
    let Ok(Feed::Changed(c)) = feed.try_recv() else {
        panic!("the create is told to the open pages");
    };
    assert_eq!((c.coll.as_str(), c.version), ("notes", 1));
}

/// **A run acts as its caller**: an account sees its own private records,
/// the app sees every one.
#[test]
fn a_run_acts_as_its_caller() {
    let env = env();
    let add = "export default async function add(ctx) { return (await ctx.data.collection('notes').create({ text: 'mine' })).id; }";
    let count = "export default async function count(ctx) { return (await ctx.data.collection('notes').list()).docs.length; }";
    let files = [("add", add), ("count", count)];
    go(&env, &files, "add", json!({}), yen());
    let mut mai = yen();
    mai.account.id = "2".into();
    assert_eq!(
        go(&env, &files, "count", json!({}), mai).answer.unwrap(),
        json!(0)
    );
    assert_eq!(
        go(&env, &files, "count", json!({}), yen()).answer.unwrap(),
        json!(1)
    );
    let app = Caller::app("home", "Home");
    assert_eq!(
        go(&env, &files, "count", json!({}), app.clone())
            .answer
            .unwrap(),
        json!(1)
    );
    let made = go(&env, &files, "add", json!({}), app).answer.unwrap();
    let owner: String = env
        .store
        .conn()
        .query_row(
            "SELECT owner FROM docs WHERE id = ?1",
            [made.as_str().unwrap()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(owner, "app:home");
}

/// **A refused write throws with its code and field; an error names its
/// line; a missing workflow is said.**
#[test]
fn what_goes_wrong_is_said_where() {
    let env = env();
    let bad = "export default async function bad(ctx) {\n\
                 try { await ctx.data.collection('notes').create({ txt: 1 }); }\n\
                 catch (e) { return [e.code, e.field, e.message]; }\n\
               }";
    let answer = go(&env, &[("bad", bad)], "bad", json!({}), yen())
        .answer
        .unwrap();
    assert_eq!(
        answer,
        json!(["invalid", "txt", "txt is not a field of this collection"])
    );
    let throws =
        "export default async function t(ctx) {\n  const x = 1;\n  throw new Error('no ' + x);\n}";
    let failed = go(&env, &[("t", throws)], "t", json!({}), yen())
        .answer
        .unwrap_err();
    assert_eq!(failed.to_string(), "apps/home/workflows/t.js:3 no 1");
    let none = go(&env, &[("t", throws)], "nope", json!({}), yen())
        .answer
        .unwrap_err();
    assert_eq!(none.to_string(), "there is no workflow `nope`");
}

/// **A loop that never ends is stopped at the deadline, and said so.**
#[test]
fn a_loop_that_never_ends_is_stopped() {
    let env = env();
    let spin = "export default async function spin(ctx) { while (true) {} }";
    let m = manifest("deadline: 1s\n");
    let started = Instant::now();
    let out = run(
        &env,
        m,
        sources(&[("spin", spin)]),
        ask("spin", json!({}), yen()),
    );
    assert!(started.elapsed() < Duration::from_secs(5));
    assert_eq!(
        out.answer.unwrap_err().to_string(),
        "ran past its deadline of 1 s and was stopped"
    );
}

/// **`ctx.once` does its work once for a key, across runs; `ctx.run` awaits
/// another workflow; `ctx.schedule` leaves a job for later.**
#[test]
fn once_run_and_schedule() {
    let env = env();
    let send = "export default async function send(ctx, { key }) {\n\
                  return ctx.once(key, async () => { ctx.log('sent'); return 'sent at ' + key; });\n\
                }";
    let outer =
        "export default async function outer(ctx) { return ctx.run('send', { key: 'k2' }); }";
    let later = "export default async function later(ctx) {\n\
                   const id = await ctx.schedule(new Date(5000), 'send', { key: 'k3' });\n\
                   try { await ctx.schedule(5000, 'nope'); } catch (e) { return [id.length, e.code]; }\n\
                 }";
    let files = [("send", send), ("outer", outer), ("later", later)];
    let first = go(&env, &files, "send", json!({"key": "k1"}), yen());
    assert_eq!(
        (first.answer.unwrap(), first.log),
        (json!("sent at k1"), vec!["sent".to_string()])
    );
    let again = go(&env, &files, "send", json!({"key": "k1"}), yen());
    assert_eq!(
        (again.answer.unwrap(), again.log),
        (json!("sent at k1"), Vec::<String>::new())
    );
    assert_eq!(
        go(&env, &files, "outer", json!({}), yen()).answer.unwrap(),
        json!("sent at k2")
    );
    let out = go(&env, &files, "later", json!({}), yen());
    assert_eq!(out.answer.unwrap(), json!([20, "no_workflow"]));
    assert!(out.scheduled);
    let jobs = list(&env.store.conn(), Some("home"), None).unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(
        (
            jobs[0].workflow.as_str(),
            jobs[0].next_ms,
            jobs[0].source.as_str(),
            jobs[0].run_as.as_deref()
        ),
        ("send", Some(5000), "workflow", Some("1"))
    );
    assert_eq!(jobs[0].args, json!({"key": "k3"}));
}

/// **`ctx.notify` writes to the inbox of whoever it names — only for an app
/// that says `notify: true`.**
#[test]
fn a_workflow_notifies() {
    let env = env();
    env.store
        .conn()
        .execute(
            "INSERT INTO accounts (id, provider, email, name, status, created_ms, seen_ms) \
             VALUES ('1', 'google', 'yen@example.com', 'Yen', 'active', 0, 0)",
            [],
        )
        .unwrap();
    let tell = "export default async function tell(ctx) {\n\
                  return ctx.notify('admins', { title: 'Mai finished the dishes', open: '/a/home/' });\n\
                }";
    let files = [("tell", tell)];
    let quiet = go(&env, &files, "tell", json!({}), yen())
        .answer
        .unwrap_err();
    assert_eq!(
        quiet.to_string(),
        "apps/home/workflows/tell.js:2 this app does not send notifications: add `notify: true` to its app.yaml"
    );
    let out = run(
        &env,
        manifest("status: live\nnotify: true\n"),
        sources(&files),
        ask("tell", json!({}), yen()),
    );
    assert_eq!(out.answer.unwrap(), json!({"accounts": 1, "devices": 0}));
    let title: String = env
        .store
        .conn()
        .query_row("SELECT title FROM inbox WHERE account = '1'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(title, "Mai finished the dishes");
}

/// **A workflow reaches nothing but `ctx`**: the host function is not left
/// on the global object, and nothing outside the app's workflows imports.
#[test]
fn a_workflow_reaches_nothing_but_ctx() {
    let env = env();
    // Not even as it loads, before its function runs.
    let peek = "const early = typeof globalThis.__yen;\n\
                export default async function peek(ctx) { return [early, typeof globalThis.__yen, typeof globalThis.__yen_run]; }";
    let answer = go(&env, &[("peek", peek)], "peek", json!({}), yen())
        .answer
        .unwrap();
    assert_eq!(answer, json!(["undefined", "undefined", "undefined"]));
    let reach =
        "export default async function reach(ctx) { return (await import('yen:ctx')).ctx.app; }";
    let failed = go(&env, &[("reach", reach)], "reach", json!({}), yen())
        .answer
        .unwrap_err();
    assert!(
        failed
            .message
            .contains("a workflow imports only the app's other workflow files"),
        "{failed}"
    );
}
