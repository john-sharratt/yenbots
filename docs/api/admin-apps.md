# Apps and every account's records

The admins' routes for the apps (`candle/docs/yenbot_app_host.md` §12.4): who
may open each, where it sits on the landing page, and what it holds. Every
one answers 403 `forbidden` to a caller who is not an admin.

An app is **public** — every active account may open it — or **private**,
opened only by those it is shared with. A `draft` app (its `app.yaml`'s
`status`) opens for the admins alone, whatever its visibility.

## Routes

### `GET /api/v1/admin/apps`

Every app the live checkout holds, with its visibility, shares, place and
collections.

```http
GET /api/v1/admin/apps

200
{"apps": [{"id": "recipes", "name": "Recipes", "status": "live", "collections": ["dishes", "lists"],
           "access": {"private": false, "position": 0, "shares": []}},
          {"id": "money", "name": "Money", "status": "draft", "collections": ["budget", "spent"],
           "access": {"private": true, "position": null, "shares": ["account:2207…"]}}]}
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
