# Templates

Working apps a new one starts from (`candle/docs/yenbot_app_host.md` §16), in
`templates/`. Each passes the check as a new app — a test holds every one to
it, built to the current edition of `docs/building-an-app.md` — and each is a
draft until Yen says otherwise. How to start from one is in the building
guide; this page says what each holds, so the closest is chosen quickly.

| Template | Holds | Shows how to |
|---|---|---|
| `blank` | `notes` (private: `text`) | one page: a field and a button that add, a `<yen-list>` with a remove on each row |
| `list` | `items` (shared: `text`, `done`, `position`) | tick off with `<yen-checkbox>`, reorder by trading `position` with a neighbour, an edit page with `<yen-form>`, a component raising events its page acts on |
| `journal` | `entries` (private: `day`, `text`, `photo`) | a photo chosen with `<yen-image-picker>` and kept with `app.files.upload` before the record that names it; a card with its thumbnail; a date picker |
| `collection` | `items` (shared: `title`, `notes`, `tags`, `photo`) | full-text search (`collection.search`), tags as filters, cards with photos, editing with `<yen-form>` |
| `planner` | `plans` (shared: `title`, `day`, `time`, `notes`) | `<yen-tabs>` between a week and a month, `<yen-calendar>` with marks, a page reading `?day=` from its address |
| `tracker` | `entries` (private: `day`, `value`, `note`) | `<yen-chart kind="line">` from a computed store, a number formatted as money |
| `assistant` | — (`ai: true`) | `<yen-chat>` with the answer arriving as it is written, the app's own system prompt in its store |
| `picture-maker` | `pictures` (private: `prompt`, `file`) | `<yen-draw>` with progress, a picture kept as a record naming its file, `<yen-photo-grid>` |
| `reminders` | `reminders` (private: `title`, `due_ms`) | a `from:` schedule — a job per record at its time — and the workflow that notifies the record's owner, sent once; turning pings on from a page |
| `daily-digest` | `items` (shared: `title`, `due_ms`) | an `every:` schedule that gathers and sends one note, run now from a button with `app.workflows.run` |
| `form` | `answers` (shared: `name`, `day`, `rating`, `bringing`) | a form of selects and fields, a thank-you, and a summary with `<yen-chart kind="bar">` worked out in a store |

## What every one has

`app.yaml`, `index.html`, `app.js` and `style.css`; its pages in `pages/`,
its stores in `stores/`, a component in `components/` where one is reused,
and a workflow in `workflows/` where it runs on its own. Each page lays
itself out in its own `static styles`; `style.css` holds page-wide rules
only, and is empty until the app needs one.

## Changing a template

A template is an app like any other: the same shape, the same rules, the
same check — the template test (`src/check/templates_tests.rs`) runs it on
every build. A change that makes a template better for every app built from
it after is welcome; the apps already built from it keep what they have.
