# Working in this repository

Read this first, every session. **Before every new app, read
`docs/building-an-app.md` in full** — the gate refuses an app not built to its
current edition. Then `docs/index.md`, and only the docs the task needs.

## What this is

**Yen Bots** (yenbot.com.au) is where Yen builds little apps for her family:
recipes, chores, reminders, a homework helper. One Rust binary, `yenbot`,
serves the whole site — the landing page, every app, the API, the store — on
loopback behind the Tokera gateway, supervised by keep on BlackWorld.

The design is `candle/docs/yenbot_app_host.md` (in the candle repository). It
is authoritative: where this repository disagrees with it, the code is wrong
unless the design is, and then the design is fixed in the same change.

## Who you are working for

**Yen is not a coder.** She works only in a chat app, through the Zen
connector, often on her phone, never at a terminal. Speak plainly. Never show
her code. Show her the result — the app's address — and ask before anything
she would not expect: deleting, making an app public, a change she did not
ask for. She shows you a page by pasting a screenshot and saying what it is.

## The map

| Path | What | Changing it |
|---|---|---|
| `apps/<id>/` | one folder per app (`docs/framework.md`) | an edit, live within a second |
| `templates/` | the apps a new one starts from (`docs/templates.md`) | an edit |
| `library/` | the art library: pictures and `catalogue.yaml` (`docs/library.md`) | a Zen task: a picture and its line, together |
| `docs/` | the manual; `docs/api/` the API's reference | an edit |
| `web/` | the platform's web files: landing and admin pages, the framework, the design system — embedded in the binary | a release |
| `src/` | the platform, one concern per file | a release |
| `yenbot.yaml` | the service's config: admins, registration, Yen's time zone, paths | an edit, then a restart |

An app's own files are read from the live checkout as `main` holds them;
everything under `web/` and `src/` is the binary's, and changes with a
release (`docs/publishing.md`).

## The order of work

1. **Read the journal**: the app's entries and Yen's taste, in the `yenbot`
   journal (`journal_grep`, `docs/journal.md`).
2. **For a new app, read `docs/building-an-app.md` in full**, start from a
   copy of the closest template, and set `guide:` to its edition.
3. **Build in the conventions**: the app's shape, its pages through stores,
   its styles in tokens (`docs/check.md`).
4. **Edit through `file_read` and `file_edit_and_commit`**: it answers
   **live** or **refused**; on a refusal nothing changed — fix what it names
   and call again.
5. **Ask Yen to look.** Note what was decided in the journal.
6. **Anything bigger** — a new app from a large template, new art, renaming,
   the platform, anything that is not a handful of an app's own text files —
   goes to a Zen task, briefed fully and watched to its end
   (`docs/zen-tasks.md`).

## The rules, each with its reason

- **Tokens only** — `var(--yen-…)`, never a literal colour, size or font:
  light and dark, an app's theme and the platform's look then reach every app
  without an edit.
- **Nothing from outside the site**: the content policy blocks it anyway, and
  a page that loads from elsewhere breaks when elsewhere does.
- **A component never reaches into a store**: it takes properties and raises
  events, so it shows whatever it is given and works anywhere.
- **One component per file, named for its tag**: finding one is never a
  search.
- **Library art by id**, never an image file in an app: one house style,
  sized once, everywhere.
- **Never edit the live directory** (`D:\yenbot\live`): it follows
  `origin/main`, and a hand edit there is overwritten and never checked.
- **Never commit a secret**: keys live in the store, set on the admin page
  (`docs/integrations.md`), or in files beside the service.
- **Never set an app live or make one public without Yen.**
- **Undo by failing forward**: a new edit that puts back what was there. No
  history is rewritten.
- **Platform changes are releases; app changes are edits.**
- **No AI attribution** in a commit, a file or a doc: the work is Yen's.

## There is no command line

The connector's tools and Zen tasks do everything. A task gets a brief it can
act on alone: what to change, in full; which docs it must read first; how it
knows it is done (`docs/zen-tasks.md`). A task on the platform builds and
tests inside it — `cargo fmt --check`, `cargo clippy --all-targets -- -D
warnings`, `cargo test`, all clean, tests beside the code, one concern per
file — commits to `main` and pushes; the release is then
`service_deploy {host: BlackWorld, service: yenbot}`, watched with
`job_status` to **proven** or **rolled back**. A build with local changes is
never tried, and a binary that does not print `INFO serving` is rolled back.

## Seeing what happened

`log_list`, `log_read` and `log_grep` on the connector: the platform is
source `yenbot`, each app `yenbot/<app>` — every browser error, failed call
and workflow run is a line there (`docs/logs.md`). Everything the site stores
— records, jobs, usage, accounts — is its API, called from a Zen task with
the `yenbot` credential (`docs/calling-the-api.md`). Look before guessing.

## When something breaks

Read the app's log first; fix forward, one change at a time; restore a
backup only for lost records (`docs/lifecycle.md`); and stop and ask Yen
before anything she would not expect (`docs/when-it-breaks.md`).

## Adding a route

A route is defined once, in `src/serve/routes.rs`: its method, path, page,
parameters and body fields. The router is built from that table,
`/api/v1/openapi.json` is written from it, and `docs/api/<page>.md` must
describe it — a section headed ``### `METHOD /path` `` naming exactly what it
takes, with an example — or the tests fail.
