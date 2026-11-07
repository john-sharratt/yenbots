# Apps and every account's records

The admins' routes for the apps (`candle/docs/yenbot_app_host.md` §12.4): who
may open each, where it sits on the landing page, and what it holds. Every
one answers 403 `forbidden` to a caller who is not an admin.

An app is **public** — every active account may open it — or **private**,
opened only by those it is shared with. A `draft` app (its `app.yaml`'s
`status`) opens for the admins alone, whatever its visibility.

## Routes

### `GET /api/v1/admin/apps`

Every app the live checkout serves, with its tile, visibility, shares,
place, collections, and what it stores. An archived app is not here: it is
in [the archive](#get-apiv1adminarchive).

```http
GET /api/v1/admin/apps

200
{"apps": [{"id": "recipes", "name": "Recipes", "status": "live", "tile": "/_library/tiles/bowl-coral-256.webp",
           "collections": ["dishes", "lists"],
           "access": {"private": false, "position": 0, "shares": []},
           "storage": {"records": 214, "files": 31, "file_bytes": 18234011}},
          {"id": "money", "name": "Money", "status": "draft", "tile": null, "collections": ["budget", "spent"],
           "access": {"private": true, "position": null, "shares": ["account:2207…"]},
           "storage": {"records": 12, "files": 0, "file_bytes": 0}}]}
```

### `GET /api/v1/admin/apps/{id}/check`

The app's last check (`docs/check.md`), made each time the live checkout
moves: `ok`, or each refusal with its rule, file and line. A change Zen makes
is checked by the gate before it lands; this is the check of what arrived
without one. While the app as it now is stands refused, `serving` names the
commit whose version is served instead — `null` for an app refused the first
time it was seen, which is not served at all.

An app that passes but raises a collection's `version` is served its last
good version the same way while its records are migrated: `migrating` names
the collections still behind, and `migrations` says how far each collection's
last migration came — `running`, `done`, or `failed` with the record and why
(`docs/workflows.md`).

| Parameter | In | |
|---|---|---|
| `id` | path | the app's id |

| Status | Code | When |
|---|---|---|
| 404 | `no_app` | the live checkout holds no such app |

```http
GET /api/v1/admin/apps/recipes/check

200
{"app": "recipes", "ok": false, "commit": "9b1e…", "serving": "4f1c…",
 "refusals": [{"path": "apps/recipes/style.css", "line": 2, "rule": "tokens",
               "message": "uses the colour `#c2375b`; use a token: var(--yen-accent), var(--yen-ink), …"}],
 "migrating": [],
 "migrations": [{"coll": "dishes", "shape": 2, "done": 41, "total": 41, "state": "done",
                 "updated_ms": 1793960400000}]}
```

### `PUT /api/v1/admin/apps/{id}/visibility`

Make an app public or private.

| Parameter | In | |
|---|---|---|
| `id` | path | the app's id |
| `private` | body | whether only its shares may open it |

| Status | Code | When |
|---|---|---|
| 404 | `no_app` | the live checkout holds no such app |

```http
PUT /api/v1/admin/apps/money/visibility

{"private": true}

200
{"app": "money", "private": true}
```

### `PUT /api/v1/admin/apps/{id}/shares`

Who a private app is shared with, replacing the list.

| Parameter | In | |
|---|---|---|
| `id` | path | the app's id |
| `who` | body | `account:<id>` and `group:<name>` |

| Status | Code | When |
|---|---|---|
| 400 | `bad_share` | an entry is neither `account:<id>` nor `group:<name>` |

```http
PUT /api/v1/admin/apps/money/shares

{"who": ["account:2207…", "group:household"]}

200
{"app": "money", "shares": ["account:2207…", "group:household"]}
```

### `PUT /api/v1/admin/apps/order`

The apps' order on the landing page: those named first, in this order; the
rest after them by name.

| Field | In | |
|---|---|---|
| `order` | body | app ids, first first |

```http
PUT /api/v1/admin/apps/order

{"order": ["recipes", "money"]}

200
{"order": ["recipes", "money"]}
```

### `GET /api/v1/admin/data/{app}/{coll}`

Any collection, every account's records whoever they belong to, read-only:
the same query as [`GET /api/v1/data/{app}/{coll}`](data.md), without the
caller's visibility.

| Parameter | In | |
|---|---|---|
| `app` | path | the app's id |
| `coll` | path | the collection |
| `where` | query | as for a query |
| `order` | query | a field, or `-field` for descending |
| `limit` | query | records per page, 1 to 200 |
| `after` | query | the cursor the last page answered |

```http
GET /api/v1/admin/data/notes/notes?order=-text&limit=1

200
{"docs": [{"id": "01HJ…", "owner": "2207…", "version": 1, "created_ms": 1793960300000, "updated_ms": 1793960300000,
           "data": {"text": "Mai's school trip"}}],
 "after": "5b224d6169…"}
```

## Deleting, archiving and restoring

Deleting an app is removing its folder from the repository. The site sees an
app with records, files or jobs and no folder, and **archives** it: it is
served to no one, its waiting jobs are held, and its records, files and jobs
are kept thirty days, then purged by the platform's nightly `purge` job.
Its folder coming back — read from the commit before it was removed and
written again — restores it as it lands. An admin may also archive an app
whose folder stays: it is hidden until restored, and kept as long as its
folder is there.

Renaming an app is moving its folder to the new id and adding
`renamed_from: <old>` to its `app.yaml`, in one commit: its records, files,
jobs and access move to the new id once, and `/a/<old>/…` redirects.

### `POST /api/v1/admin/apps/{id}/archive`

Archive an app: hidden from everyone, its waiting jobs held, everything of
it kept.

| Parameter | In | |
|---|---|---|
| `id` | path | the app's id |

| Status | Code | When |
|---|---|---|
| 404 | `no_app` | no such app is served |

```http
POST /api/v1/admin/apps/money/archive

200
{"app": "money", "archived": true}
```

### `POST /api/v1/admin/apps/{id}/restore`

Serve an archived app again: the jobs archiving held wait again.

| Parameter | In | |
|---|---|---|
| `id` | path | the app's id |

| Status | Code | When |
|---|---|---|
| 409 | `no_folder` | `apps/<id>/` is not in the repository: bring it back first |
| 404 | `not_archived` | the app is not archived |

```http
POST /api/v1/admin/apps/money/restore

200
{"app": "money", "restored": true}
```

### `GET /api/v1/admin/archive`

The archived apps, oldest first: why each is archived (`removed`, its folder
gone, or `admin`), since when, when it is purged (`null` while its folder
is there), and how many records and files it holds.

```http
GET /api/v1/admin/archive

200
{"archived": [{"app": "money", "reason": "removed", "archived_ms": 1793960000000,
               "purge_ms": 1796552000000, "records": 214, "files": 3}]}
```

## Use, errors and feedback

### `GET /api/v1/admin/usage`

Which apps get used: by app and day — or by app and account — the times it
was opened, the accounts that opened it, and what it asked the model
(`chats`, `questions`) and the image engine (`pictures`). `key` is the day,
or the account's id (`""` for a workflow running as the app).

| Parameter | In | |
|---|---|---|
| `app` | query | only this app's |
| `since` | query | from this day, `YYYY-MM-DD` in site time; four weeks ago when absent |
| `by` | query | `day` or `account`; `day` when absent |

| Status | Code | When |
|---|---|---|
| 400 | `bad_request` | `by` is neither, or `since` is not a day |

```http
GET /api/v1/admin/usage?app=recipes&since=2026-11-01

200
{"since": "2026-11-01", "by": "day",
 "usage": [{"app": "recipes", "key": "2026-11-06", "opens": 9, "accounts": 3, "chats": 2, "pictures": 1, "questions": 0},
           {"app": "recipes", "key": "2026-11-07", "opens": 4, "accounts": 2, "chats": 0, "pictures": 0, "questions": 1}]}
```

### `GET /api/v1/admin/errors`

What browsers reported going wrong (`docs/logs.md`), newest first: each
line as the app's log holds it.

| Parameter | In | |
|---|---|---|
| `app` | query | only this app's; `_site` for the platform's own pages |
| `limit` | query | how many, 1 to 500; 100 when absent |

```http
GET /api/v1/admin/errors?app=recipes&limit=1

200
{"errors": [{"at_ms": 1793960400000, "source": "yenbot/recipes",
             "message": "TypeError: r.title is undefined — at https://yenbot.com.au/a/recipes/r/7 — account 2207… on Safari 18"}]}
```

### `GET /api/v1/admin/feedback`

The notes people sent from the apps' menus, newest first, with who sent
each.

| Parameter | In | |
|---|---|---|
| `app` | query | only this app's |
| `limit` | query | how many, 1 to 500; 100 when absent |

```http
GET /api/v1/admin/feedback?app=chores

200
{"feedback": [{"app": "chores", "account": "2207…", "name": "Mai", "text": "Can it remind me too?",
               "at_ms": 1793960400000}]}
```
