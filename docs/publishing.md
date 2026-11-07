# Editing and publishing

How a change reaches the site (`candle/docs/yenbot_app_host.md` §5.4, §5.5).

## An edit

Every change to an app, a template or a doc is a commit made with
`file_edit_and_commit` on the yenbot repository — several files in one
commit when they belong together. Before `main` moves, the gate — yenbot
itself — checks the change (`docs/check.md`) and answers:

- **live**: it is on `main`, and on the site within a second; every open
  page of the app reloads itself, so Yen sees it without touching her phone.
- **refused**: nothing moved. The answer names the file, the line and what to
  do instead. Fix exactly that and call again — never work around it.

`file_read` reads a file as `main` holds it. Never edit the live directory
(`D:\yenbot\live`): it follows `origin/main`, and a hand edit there is
overwritten and never checked.

## Drafts and live

An app is born a draft (`status: draft`): the admins see it and open it;
nobody else does. Going live is one edit — `status: live` — made only when
Yen says so. Then every active account sees it, unless she made it private.

## A release

A change to the platform — `src/`, `web/` (the framework, the design system,
the landing and admin pages), `Cargo.toml` — is a new binary:

1. A Zen task makes it, with tests, and pushes it to `main`
   (`docs/zen-tasks.md`).
2. `service_deploy {host: BlackWorld, service: yenbot}` builds it there and
   has keep try it; `job_status` on the answer's handle waits for it —
   **proven** (it serves) or **rolled back** (the old binary is back).
3. Confirm it landed: `GET /v1/status` answers the build it runs.

A release takes a few minutes. Tell Yen roughly how long before starting.
Every app is checked against a changed platform component the next time it
changes.

## Undo

Undo is failing forward: a new edit that puts back what was there, read from
the earlier commit (a Zen task reads history; `docs/zen-tasks.md`) or from
what the journal noted. History is never rewritten.
