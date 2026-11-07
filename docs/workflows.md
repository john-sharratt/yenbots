# Jobs and workflows

Something an app does on its own — a reminder at its time, a summary every
morning, a note when the kids finish a chore — is a **workflow** the app
ships, run by a **job** (`candle/docs/yenbot_app_host.md` §9, §10). A
workflow is a file in the app's `workflows/`; the jobs are declared in its
`app.yaml`, or made by a workflow for later. Both are live on the next edit,
with no release.

## A workflow

```js
// apps/reminders/workflows/remind.js
export default async function remind(ctx, { record }) {
  ctx.log("reminding", record.owner, "of", record.title);
  return record.title;
}
```

One file, one `export default async function`, named for its file. It may
import the app's other workflow files by a relative path (`./shared.js`) and
nothing else: there is no filesystem, no process and no network — everything
it reaches is on `ctx`. The check compiles every workflow before it lands and
refuses one that does not compile, exports no function, imports from outside
`workflows/`, or does work as it loads (`docs/check.md`).

A run is its own: nothing is kept between runs but what a workflow writes to
the store. It has a minute to finish — `deadline: 5m` in `app.yaml` gives
every workflow of the app longer, up to ten minutes — and 64 MiB; a loop
that never ends is stopped at the deadline and said so.

### `ctx`

| `ctx.` | |
|---|---|
| `app` | the app's id |
| `account` | the account the run acts for, or `null` when it acts as the app |
| `job` | the job being run, `{id, due_ms}`, or `null` for a run asked for at once |
| `data.collection(name)` | one of the app's collections: `list({where, order, limit, after})`, `get(id)`, `create(fields)`, `update(id, fields, {version})`, `delete(id)`, `sum({where, group, of})`, `search(q, {limit})` |
| `ai.chat(ask)`, `ai.image(draw)`, `ai.ask(from, question)` | the model and the image engine, answered whole, for an app with `ai: true` (`ai.md`); a picture is kept as a file of the app, `{file, seed}` |
| `notify(to, {title, body, open})` | a notification to `to` — an account's id, `group:<name>`, or `admins` — written to each one's inbox and pushed to their devices; only those who may open the app are told; `open` is the page a tap opens (`/a/<app>/…`, the app's start when absent); answers `{accounts, devices}`; the app's `app.yaml` says `notify: true` |
| `schedule(at, workflow, args)` | a job for later — `at` in milliseconds or a `Date` — acting for the same account; answers its id |
| `run(workflow, args)` | another of the app's workflows, run in this run and awaited |
| `once(key, fn)` | `fn` run once for `key`, whatever retries come; its answer is kept and given again |
| `time(ms)` | the time as Yen reads it, in her zone: `Sunday 8 November, 7:30 pm` |
| `now()` | the time now, in milliseconds |
| `log(...)` | a line in the run's log |

A record reads as a page reads it: its fields beside `id`, `owner`,
`version`, `created_ms` and `updated_ms`. A run acting for an account sees
and changes what that account may, exactly as the API judges it; a run
acting as the app reaches every record of the app's own collections, and a
record it makes is owned by `app:<id>`. A refused call throws an `Error` with
its `code` (`invalid`, `not_found`, `forbidden`, `stale`, …) and, for a field
a collection refuses, its `field`. Every write is told to the open pages and
keeps the app's jobs in step, as a write through the API does.

**Sending something** — a notification, a message — goes inside `ctx.once`,
keyed on the job and its time, so a run repeated after a crash does not send
it twice:

```js
// apps/meals/workflows/plan-meals.js — `every: "weekly sun 19:00"`
export default async function planMeals(ctx) {
  await ctx.once(`${ctx.job.id}@${ctx.job.due_ms}`, () =>
    ctx.notify("admins", { title: "Time to plan the meals", body: "What's for dinner this week?", open: "/a/meals/" }),
  );
}
```

A notification is the inbox's first, then a push to each device the person
turned notifications on for — unless they stopped the app's pushes from its
bell. A message is a title (up to 120 characters), a body (up to 1,000) and
the page it opens.

## Schedules

```yaml
schedules:
  - every: "weekly sun 19:00"   # a repeat rule
    run: plan-meals
  - from: reminders             # one job per record,
    at: due_ms                  #   at this `time` field,
    before: 10m                 #   less this
    run: remind
  - when: { collection: chores, does: update, where: { done: true } }
    run: chore-done
```

**`every:`** — a repeat rule, in Yen's time zone (`yenbot.yaml`'s
`time_zone`):

| Rule | Runs |
|---|---|
| `hourly` | on the hour |
| `daily 07:30` | every day at 07:30 |
| `weekly sun 19:00`, `weekly mon,thu 09:00` | on those days (`sun` `mon` `tue` `wed` `thu` `fri` `sat`) |
| `monthly 1st 08:00` | that day of every month, or its last day in a month too short |
| `*/15 7-21 * * *` | a cron line: minute, hour, day, month, weekday |

The workflow is handed `{}`. A time the clocks skip runs just after the
change; one they pass twice runs once.

**`from:`** — one job per record of the collection whose `at` field holds a
time still to come, less `before` (`30s`, `10m`, `2h`, `1d`). Creating,
moving or deleting the record makes, moves or ends its job, in the same
write. The workflow is handed `{record}`, the record as it is when the job
runs.

**`when:`** — a job, due at once, for each write that matches: on `create`,
the new record matches `where`; on `delete`, the record as it was; on
`update`, the record comes to match — it did not before and does after — so
a chore marked done runs it once, and an edit to a chore already done does
not. The workflow is handed `{before, after}`, either `null` for a create or
a delete. It runs moments after the write, which never waits on it.

**When the site was down**: a job due within the last hour runs once when
it comes back; an older one runs only if its schedule says `catch_up: true`
(an `every:` alone), and a repeating job then moves to its next time rather
than running every one it missed. A job passed over keeps a failed run that
says so.

## Running, failing, and seeing what happened

A run that fails is tried again after 1, 5 and 30 minutes; after the fourth
failure a job that runs once is `failed`, and a repeating one waits for its
next time. Every run is kept with its job — when, how it went, the error with
its file and line, what it logged — and written to the app's log
(`yenbot/<app>`, target `workflow`):

```text
2026-11-08T08:00:00.051Z  INFO yenbot/meals workflow: workflow plan-meals (job 19a5…) ran in 51 ms
planned 7 dinners
```

- `GET /api/v1/jobs/<app>` — the app's jobs and their last runs
  (`api/jobs.md`); `GET /api/v1/admin/jobs` every job, for an admin
  (`api/admin-jobs.md`).
- `POST /api/v1/admin/jobs/<id>/run` runs a job now; `hold` and `release`
  stop and start it.
- `POST /api/v1/workflows/<app>/<name>` runs a workflow now as the caller —
  a "send it now" button, `app.workflows.run(name, args)` from a page — and
  `POST /api/v1/admin/workflows/<app>/<name>` as the app. A workflow the
  platform hands a record (a `from:` or `when:` schedule's) or a migration is
  not a person's to run, as they could hand it a record of their own making:
  a button runs a workflow of its own, which may `ctx.run` the shared work.

## Changing a collection's shape

A collection's `version` (§7.5) rises when its records change shape — a field
renamed, split, or given a new meaning. Each step ships a migration,
`workflows/migrate-<coll>-<n>.js`, that takes one record at shape `n - 1` and
returns its fields at shape `n`:

```js
// apps/recipes/workflows/migrate-recipes-2.js — `name` became `title`
export default async function migrate(ctx, { record }) {
  const { name, ...rest } = record;
  return { ...rest, title: name };
}
```

The check refuses a raised version without each step's migration, and a
version lowered. When the change lands, the site takes every record up, step
by step, a hundred at a time, while the app goes on being served as it was;
once none is behind, the app is served as it now is and its open pages
reload. A record taken to the new version is held to its fields as any write
is; one refused stops the migration, the app stays as it was, and
`GET /api/v1/admin/apps/<id>/check` says which record and why — a new commit
with the migration fixed tries again.
