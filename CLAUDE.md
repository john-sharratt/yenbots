# CLAUDE.md

Read this first, every session. Then `docs/index.md`, and only the docs the task needs.

## What this is

**Yen Bots** (yenbot.com.au) is where Yen builds little apps for her family. One Rust
binary, `yenbot`, serves the whole site — the landing page, every app, the API, the store
— on loopback behind the Tokera gateway, supervised by keep on BlackWorld.

The design is `candle/docs/yenbot_app_host.md` (in the candle repository). It is
authoritative: where this repository disagrees with it, the code is wrong unless the
design is, and then the design is fixed in the same change.

## Who you are working for

**Yen is not a coder.** She works only in claude.ai, through the Zen connector, often on
her phone. Speak plainly, never show her code, show her the result, and ask before doing
anything she would not expect. She shows you a page by pasting a screenshot and saying
what it shows.

## The map

| Path | What | Changing it |
|---|---|---|
| `src/` | the platform: one concern per file | a release (`service_deploy`) |
| `web/` | the platform's web files: landing page, framework, design system | an edit; the gateway serves it from disk, the binary embeds it |
| `apps/<id>/` | one folder per app | an edit, live within a second |
| `library/` | the art library | an edit |
| `templates/` | starting points for a new app | an edit |
| `docs/` | the manual | an edit |
| `yenbot.yaml` | the service's config: admins, registration, paths | an edit, then a restart |

## The order of work

1. Read the app's journal entries (the `yenbot` journal).
2. For a new app, read `docs/building-an-app.md` in full first.
3. Edit through `file_read` and `file_edit_and_commit`: it answers **live** or **refused**,
   and on a refusal nothing changed — fix what it names and call again.
4. Ask Yen to look. Note what was decided in the journal.
5. Anything bigger — the Rust, the framework, renaming, a large template — goes to a Zen
   task, briefed fully and watched to its end; a change to `src/` is then released with
   `service_deploy {host: BlackWorld, service: yenbot}`.

## Rules, each with its reason

- **There is no command line.** The connector's tools and Zen tasks do everything.
- **Never edit the live directory** (`D:\yenbot\live`): it follows `origin/main`, and a
  hand edit there is overwritten and never reviewed.
- **Never commit a secret.** Keys live in the store or in files beside the service.
- **Never set an app live or make one public without Yen.**
- **Undo by failing forward**: a new edit that puts back what was there. No history is
  rewritten.
- **No AI attribution** in a commit, a file or a doc: the work is Yen's.
- **One concern per file; tests beside the code**; `cargo fmt`, `cargo clippy
  --all-targets -- -D warnings` and `cargo test` clean before a commit.
- **`--build-info` and `INFO serving`** are what keep checks: a build with local changes
  is never tried, and a binary that does not print `INFO serving` on stderr is rolled back.

## Seeing what happened

`log_list`, `log_read` and `log_grep` on the connector: the platform is source `yenbot`,
each app `yenbot/<app>`. Everything the site stores is its API, called from a Zen task
with the HTTP tools (`docs/calling-the-api.md`). Look before guessing.
