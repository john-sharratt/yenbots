# Jobs and workflows

An app's jobs and its workflows, as its own pages reach them
(`candle/docs/yenbot_app_host.md` §9, §10; how to write them:
`docs/workflows.md`). A job is a workflow due to run — from a repeat rule, a
record's time, a write, or a workflow that scheduled it — and a workflow is
an `export default async function` in the app's `workflows/`.

The admins see every job, and run, hold and release them, on
[admin-jobs](admin-jobs.md).

## Routes

### `GET /api/v1/jobs/{app}`

The app's jobs the caller's pages may show: those acting for the caller, and
those acting as the app itself. Each says when it next runs and how its last
five runs went, newest first. A job's arguments are not shown here, as they
may hold a record.

| Parameter | In | |
|---|---|---|
| `app` | path | the app's id |

| Status | Code | When |
|---|---|---|
| 404 | `no_app` | no such app, or none the caller may open |

```http
GET /api/v1/jobs/meals

200
{"jobs": [{"id": "19a5f3c2e0000001a2b", "app": "meals", "workflow": "plan-meals",
           "source": "schedule:0", "rule": "weekly sun 19:00", "next_ms": 1794042000000,
           "state": "waiting", "attempts": 0, "run_as": null,
           "runs": [{"job": "19a5f3c2e0000001a2b", "app": "meals", "workflow": "plan-meals",
                     "started_ms": 1793437200012, "ended_ms": 1793437200064, "ok": true,
                     "log": ["told 2 people"]}]}]}
```

### `POST /api/v1/workflows/{app}/{name}`

Run a workflow now, as the caller — a "send me today's summary now" button.
The body is the workflow's arguments, an object; no body is `{}`. It answers
when the run ends, with what the workflow returned, the lines it logged with
`ctx.log`, and how long it took. The run is kept, and written to the app's
log, as a job's is.

| Parameter | In | |
|---|---|---|
| `app` | path | the app's id |
| `name` | path | the workflow, a file in the app's `workflows/` |

| Status | Code | When |
|---|---|---|
| 404 | `no_app` | no such app, or none the caller may open |
| 404 | `no_workflow` | the app has no workflow of that name |
| 403 | `not_yours_to_run` | the platform hands this workflow a record — a `from:` or `when:` schedule's, or a migration (`migrate-…`) — so a person may not run it with arguments of their own; an admin runs it as the app on [admin-jobs](admin-jobs.md) |
| 400 | `bad_request` | the body is not an object |
| 422 | `workflow_failed` | the workflow threw, or ran past its deadline: `detail` says what, with its file and line, and `log` what it logged first |

```http
POST /api/v1/workflows/meals/plan-meals

{"week": "2026-11-09"}

200
{"value": {"planned": 7}, "log": ["planned the week of 2026-11-09"], "ms": 48}
```
