# Jobs, for the admins

Every job on the site, and what an admin does to one
(`candle/docs/yenbot_app_host.md` §6.2, §9). Every route here answers 403
`forbidden` to a caller who is not an admin.

A job is `waiting` (for `next_ms`), `running`, `held` (by an admin), `done`
(it ran once, and is not due again) or `failed` (its tries spent). A run that
fails is tried again after 1, 5 and 30 minutes; after the fourth failure a
job that runs once is `failed`, and a repeating one moves on to its next
time. Its runs say what happened each time: when, whether it went well, the
error with its file and line, and what it logged.

`source` says what made it: `schedule:<n>` (the manifest's `n`th schedule, a
repeat rule), `record:<n>:<coll>/<id>` (that schedule's job for one record),
`write:<coll>/<id>` (a write a `when:` heard), or `workflow` (`ctx.schedule`).
The platform's own jobs — the nightly `backup` at 03:00 and `tidy` at 04:00 —
belong to the app `_platform`.

## Routes

### `GET /api/v1/admin/jobs`

Every job, soonest first, with its arguments and its last five runs.

| Parameter | In | |
|---|---|---|
| `app` | query | only this app's; `_platform` for the platform's own |
| `state` | query | only those `waiting`, `running`, `held`, `done` or `failed` |

| Status | Code | When |
|---|---|---|
| 400 | `bad_state` | `state` is not one of the five |

```http
GET /api/v1/admin/jobs?app=reminders

200
{"jobs": [{"id": "19a5f3c2e0000001a2c", "app": "reminders", "workflow": "remind",
           "source": "record:0:reminders/19a5f1aa000000019f", "rule": null,
           "next_ms": 1794006000000, "state": "waiting", "attempts": 0, "run_as": "1170…",
           "args": {"coll": "reminders", "id": "19a5f1aa000000019f"}, "runs": []}]}
```

### `POST /api/v1/admin/jobs/{id}/{action}`

`run` makes the job due now — the scheduler runs it within moments, and a
repeating job then moves on to its next time; `hold` stops a waiting job
from running until it is released; `release` lets it run again. It answers
the job as it is after.

| Parameter | In | |
|---|---|---|
| `id` | path | the job's id |
| `action` | path | `run`, `hold` or `release` |

| Status | Code | When |
|---|---|---|
| 404 | `no_action` | the action is not `run`, `hold` or `release` |
| 404 | `no_job` | there is no such job |
| 409 | `refused` | it is running now, or — to hold — not waiting, or — to release — not held |

```http
POST /api/v1/admin/jobs/19a5f3c2e0000001a2b/run

200
{"job": {"id": "19a5f3c2e0000001a2b", "app": "meals", "workflow": "plan-meals",
         "source": "schedule:0", "rule": "weekly sun 19:00", "next_ms": 1793960400000,
         "state": "waiting", "attempts": 0, "run_as": null, "args": {}, "runs": []}}
```

### `POST /api/v1/admin/workflows/{app}/{name}`

Run a workflow now, as the app itself — every record of its collections in
reach — answering when the run ends with what it returned and logged. The
body is its arguments, an object; no body is `{}`.

| Parameter | In | |
|---|---|---|
| `app` | path | the app's id |
| `name` | path | the workflow, a file in the app's `workflows/` |

| Status | Code | When |
|---|---|---|
| 404 | `no_app` | the live checkout serves no such app |
| 404 | `no_workflow` | the app has no workflow of that name |
| 400 | `bad_request` | the body is not an object |
| 422 | `workflow_failed` | the workflow threw, or ran past its deadline |

```http
POST /api/v1/admin/workflows/reminders/remind

{"record": {"id": "19a5f1aa000000019f", "title": "Call Mai", "due_ms": 1794006600000}}

200
{"value": null, "log": ["reminded 1170… of Call Mai"], "ms": 31}
```
