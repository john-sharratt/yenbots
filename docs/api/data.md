# An app's records

Each app keeps its records in the collections its `app.yaml` declares
(`candle/docs/yenbot_app_host.md` §7). A record is
`{id, owner, data, version, created_ms, updated_ms}`: `data` holds the fields
the collection declares and no others, and `version` counts its writes.

Who sees what:

- **A private collection's** records are their owner's, and those they share
  one with (`_share`).
- **A shared collection's** records are read by those its `roles.read`
  names — `everyone`, or groups by name — and added to, changed or deleted
  by those its `roles.write` names.
- **Anyone may change their own records**, and one shared with them to edit.

The errors these routes add to the [common ones](index.md#errors):

| Status | Code | When |
|---|---|---|
| 404 | `no_app` | there is no such app, or the caller may not open it |
| 404 | `no_collection` | the app declares no such collection |
| 404 | `not_found` | no such record, or not one the caller may see |
| 403 | `forbidden` | the caller may see it but not do this to it |
| 412 | `stale` | `If-Match` (or a batch write's `version`) names a version that is not the record's; the answer's `version` is the one held |
| 422 | `invalid` | a field is undeclared, missing, of the wrong type or out of its bounds; `field` names it, `reason` says why |
| 400 | `bad_request` | a `where`, `order`, `group` or `of` names something the collection does not have, or a cursor is not one |

## Routes

### `GET /api/v1/data/{app}/{coll}`

A page of the records the caller may see, matching `where`, in `order`. When
there are more, the answer's `after` asks for the next page.

| Parameter | In | |
|---|---|---|
| `app` | path | the app's id |
| `coll` | path | the collection |
| `where` | query | clauses `field op value` joined by ` and `: ops `=`, `!=`, `<`, `<=`, `>`, `>=`, and `~` (contains); a value is a number, `true`, `false`, `null`, or text — `"quoted"` when it holds a space or would read as one of those |
| `order` | query | a field, or `-field` for descending; creation order when absent |
| `limit` | query | records per page, 1 to 200; 50 when absent |
| `after` | query | the cursor the last page answered |

```http
GET /api/v1/data/recipes/dishes?where=minutes <= 30 and kind = soup&order=-minutes&limit=2

200
{"docs": [{"id": "01HF…", "owner": "1093…", "version": 2, "created_ms": 1793959331042, "updated_ms": 1793959400000,
           "data": {"name": "Phở", "kind": "soup", "minutes": 30}},
          {"id": "01HG…", "owner": "1093…", "version": 1, "created_ms": 1793959500000, "updated_ms": 1793959500000,
           "data": {"name": "Canh chua", "kind": "soup", "minutes": 25}}],
 "after": "5b32352c22303148…"}
```

### `POST /api/v1/data/{app}/{coll}`

Create a record. The body is its fields; a declared default fills one left
out. Answers 201 with the record.

| Parameter | In | |
|---|---|---|
| `app` | path | the app's id |
| `coll` | path | the collection |

```http
POST /api/v1/data/recipes/dishes
X-Yen-App: recipes

{"name": "Bún chả", "kind": "noodles", "minutes": 45}

201
{"id": "01HH…", "owner": "1093…", "version": 1, "created_ms": 1793960000000, "updated_ms": 1793960000000,
 "data": {"name": "Bún chả", "kind": "noodles", "minutes": 45}}
```

### `GET /api/v1/data/{app}/{coll}/_sum`

Counts, grouped by one or two fields; with `of`, the sum, average, least and
greatest of that field too.

| Parameter | In | |
|---|---|---|
| `app` | path | the app's id |
| `coll` | path | the collection |
| `where` | query | as for a query |
| `group` | query | one or two fields, comma-separated; one row in all when absent |
| `of` | query | the number field the sum, average, least and greatest are of |

```http
GET /api/v1/data/money/spent/_sum?group=category&of=amount&where=month = "2026-11"

200
{"groups": [{"category": "food", "count": 14, "sum": 412.5, "avg": 29.46, "min": 3.2, "max": 96.0},
            {"category": "school", "count": 2, "sum": 180.0, "avg": 90.0, "min": 60.0, "max": 120.0}]}
```

### `GET /api/v1/data/{app}/{coll}/_search`

Full-text search over the fields the collection names in `search`, best
first. Each word matches as the start of a word, accents and case ignored:
`pho` finds “Phở”.

| Parameter | In | |
|---|---|---|
| `app` | path | the app's id |
| `coll` | path | the collection |
| `q` | query | the words, required |
| `limit` | query | how many, 1 to 200; 20 when absent |

```http
GET /api/v1/data/recipes/dishes/_search?q=pho bo

200
{"docs": [{"id": "01HF…", "owner": "1093…", "version": 2, "created_ms": 1793959331042, "updated_ms": 1793959400000,
           "data": {"name": "Phở bò", "kind": "soup", "minutes": 30}}]}
```

A collection that names no `search` fields answers 400 `bad_request`.

### `GET /api/v1/data/{app}/{coll}/{id}`

One record.

| Parameter | In | |
|---|---|---|
| `app` | path | the app's id |
| `coll` | path | the collection |
| `id` | path | the record's id |

```http
GET /api/v1/data/recipes/dishes/01HF…

200
{"id": "01HF…", "owner": "1093…", "version": 2, "created_ms": 1793959331042, "updated_ms": 1793959400000,
 "data": {"name": "Phở bò", "kind": "soup", "minutes": 30}}
```

### `PATCH /api/v1/data/{app}/{coll}/{id}`

Merge the body's fields into the record; a field set to `null` is removed,
unless it is required. Send `If-Match` with the version you read, and a
change someone else made in between answers 412 `stale` instead of being
overwritten.

| Parameter | In | |
|---|---|---|
| `app` | path | the app's id |
| `coll` | path | the collection |
| `id` | path | the record's id |

```http
PATCH /api/v1/data/recipes/dishes/01HF…
X-Yen-App: recipes
If-Match: 2

{"minutes": 35, "note": null}

200
{"id": "01HF…", "owner": "1093…", "version": 3, "created_ms": 1793959331042, "updated_ms": 1793960100000,
 "data": {"name": "Phở bò", "kind": "soup", "minutes": 35}}
```

### `DELETE /api/v1/data/{app}/{coll}/{id}`

Delete a record, with its shares and its place in search.

| Parameter | In | |
|---|---|---|
| `app` | path | the app's id |
| `coll` | path | the collection |
| `id` | path | the record's id |

```http
DELETE /api/v1/data/recipes/dishes/01HF…
X-Yen-App: recipes

200
{"deleted": "01HF…"}
```

### `PUT /api/v1/data/{app}/{coll}/{id}/_share`

Who else may read or edit one record of a private collection, replacing what
was set. Only its owner shares it.

| Parameter | In | |
|---|---|---|
| `app` | path | the app's id |
| `coll` | path | the collection |
| `id` | path | the record's id |
| `read` | body | `account:<id>` and `group:<name>` who may read it |
| `edit` | body | those who may also change it |

```http
PUT /api/v1/data/notes/notes/01HJ…/_share
X-Yen-App: notes

{"read": ["group:household"], "edit": ["account:2207…"]}

200
{"read": ["group:household"], "edit": ["account:2207…"]}
```

### `POST /api/v1/data/{app}/_batch`

Several writes in one transaction: all of them, or — when one is refused —
none, the refusal's `x-yen-batch-at` header naming which (from 0). Each write
is `{"op": "create", "coll", "data"}`, `{"op": "update", "coll", "id", "data",
"version"?}` or `{"op": "delete", "coll", "id"}`.

| Parameter | In | |
|---|---|---|
| `app` | path | the app's id |
| `writes` | body | the writes, in order |

```http
POST /api/v1/data/money/_batch
X-Yen-App: money

{"writes": [{"op": "create", "coll": "spent", "data": {"amount": 12.5, "category": "food", "month": "2026-11"}},
            {"op": "update", "coll": "budget", "id": "01HK…", "data": {"left": 287.5}, "version": 4}]}

200
{"done": [{"id": "01HM…", "owner": "1093…", "version": 1, "created_ms": 1793960200000, "updated_ms": 1793960200000,
           "data": {"amount": 12.5, "category": "food", "month": "2026-11"}},
          {"id": "01HK…", "owner": "1093…", "version": 5, "created_ms": 1793900000000, "updated_ms": 1793960200000,
           "data": {"left": 287.5}}]}
```
