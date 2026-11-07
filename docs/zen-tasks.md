# Bigger changes: Zen tasks

What is not a handful of an app's own files goes to a Zen task
(`candle/docs/yenbot_app_host.md` §17.7): a new app from a large template,
new art for the library, renaming an app, the platform's Rust or its web
files, anything reading history, and every call to the site's API
(`docs/calling-the-api.md`).

## Starting one

`repo_task_start` on the `yenbot` repository, named for what it does, with a
brief a worker can act on alone — it has the repository, not this chat:

- **What to change**, in full: the app, the files, the behaviour, Yen's
  words where they matter.
- **What to read first**: `CLAUDE.md`, `docs/building-an-app.md` for an app,
  the docs of the parts it touches.
- **How it knows it is done**: the check passes (it is the gate); for the
  platform, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`
  and `cargo test` clean; pushed to `main`; what to answer with.

```text
In the yenbot repository, start a new app `recipes` from templates/collection,
following docs/building-an-app.md (read it in full first): id `recipes`, name
"Recipes", tile library/tiles/pot-sage, collection `recipes` with title,
serves (number), photo (file) and tags; draft. Commit it to main with
file_edit_and_commit and answer with the gate's word and the app's address.
```

## Watching it

A task runs for minutes; a claude.ai chat does not wake on its own. So:

1. Tell Yen roughly how long it will take — a few minutes for an app, ten or
   more for a release.
2. `job_status` on its handle — one call that waits — while she waits, or
   when she comes back.
3. Read its result and its commits; check what it claims against what
   landed — open the app, read `GET /v1/status`, read the log.
4. Tell Yen in plain words what changed, and ask her to look.

## A release

After a task pushes a platform change, `service_deploy {host: BlackWorld,
service: yenbot}` releases it (`docs/publishing.md`); `job_status` watches it
to **proven** or **rolled back**.

Nothing here waits on an approval: an edit, a task on the yenbot repository
and a release of the yenbot service all run on the connector's grant alone.
