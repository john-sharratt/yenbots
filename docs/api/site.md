# The site

Who the caller is, what they may open, the events an open page listens to,
the errors a page reports, and the description of this API.

## Routes

### `GET /api/v1/me`

Who the caller is. A signed-in person's first call registers their account;
until an admin approves it, its `status` is `pending` and this is the only
route that answers it. A token's call answers the token's account, with the
token's name.

```http
GET /api/v1/me
Authorization: Bearer 3f9c…

200
{"signed_in": true, "site": "Yen Bots",
 "account": {"id": "1093…", "email": "yen@example.com", "name": "Yen", "picture": null,
             "status": "active", "admin": true, "created_ms": 1793959331042, "seen_ms": 1793959400000},
 "groups": ["household"], "token": "zen"}
```

Signed out, it answers `{"signed_in": false, "site": "Yen Bots"}`.

### `GET /api/v1/apps`

The apps the caller may open, in the landing page's order: those an admin
placed first, then the rest by name. A draft app is listed to admins alone; a
private app to those it is shared with.

```http
GET /api/v1/apps

200
{"apps": [{"id": "recipes", "name": "Recipes", "description": "What we cook",
           "status": "live", "private": false,
           "tile": "/_library/tiles/bowl-coral-256.webp", "accent": "#b03a5b"}]}
```

`tile` is the app's library picture at the landing page's size, and
`accent` its theme's accent; either is `null` when the app names none.

### `GET /api/v1/apps/{app}`

One app as its pages are built from it: its collections, each with its
scope, its fields as `app.yaml` declares them, what search covers, and
whether the caller may add a record (`may_add` — always, to a private
collection; to a shared one, when its `roles.write` names the caller). The
framework's forms are built from it. An app the caller may not open is
`404 no_app`.

| Parameter | In | |
|---|---|---|
| `app` | path | the app's id |

```http
GET /api/v1/apps/chores

200
{"id": "chores", "name": "Chores", "description": "Who does what", "status": "live",
 "collections": {"chores": {"scope": "shared",
                            "fields": {"task": {"type": "text", "required": true},
                                       "done": {"type": "bool", "required": false, "default": false}},
                            "search": ["task"], "may_add": false}}}
```

### `GET /api/v1/events/_reload`

Server-sent events: one `reload` each time the site starts serving a new
commit of its repository, with the commit as its data. The platform's pages
listen and reload themselves.

```text
GET /api/v1/events/_reload

event: reload
data: 4f1c2e9
```

### `GET /api/v1/events/{app}`

Server-sent events: the app's change feed, which the framework keeps every
open page's collections current with.

- **`change`**, for each write to a record the caller may see: its
  collection, id, what happened (`created`, `updated` or `deleted`) and the
  version it is now at — never its fields, which the page reads through the
  data routes. A private record's change reaches its owner and those it is
  shared with — and, when its shares change, those it was shared with
  before. A deletion names only the id, and reaches everyone who may open
  the app.
- **`reset`**, when the page should read everything again: after a restore,
  or when it fell behind the feed.

| Parameter | In | |
|---|---|---|
| `app` | path | the app's id |

```text
GET /api/v1/events/recipes

event: change
data: {"coll":"recipes","id":"019a5f3c2e1000000a1","op":"updated","version":3}

event: reset
data: {}
```

### `POST /api/v1/errors`

What went wrong in a browser: the framework posts every uncaught error,
rejected promise and failed call. It is written to the app's log
(`yenbot/<app>`, or `yenbot` for the platform's own pages) with the page,
the account and the browser, where `log_read` and `log_grep` find it. An
error is written the first time it comes, then once a minute at most,
saying how many times it came in between. It needs a signed-in account,
approved or not, and the `X-Yen-App` header; an app there is not is
`404 no_app`.

| Field | | |
|---|---|---|
| `app` | required | the app's id, or `_site` for the platform's own pages |
| `page` | required | the page's address |
| `message` | required | what went wrong |
| `stack` | | where, as the browser tells it |
| `repeats` | | how many more times the page saw it since it last posted it |

```http
POST /api/v1/errors
X-Yen-App: recipes

{"app": "recipes", "page": "https://yenbot.com.au/a/recipes/r/7",
 "message": "TypeError: r.title is undefined", "stack": "at render (pages/recipe.js:12:5)"}

200
{"noted": "written"}
```

A repeat within the minute answers `{"noted": "counted"}`.

### `GET /api/v1/openapi.json`

This API as OpenAPI 3.1, written from the same table as this reference; its
`info.version` is the build serving it.

```http
GET /api/v1/openapi.json

200
{"openapi": "3.1.0", "info": {"title": "Yen Bots", "version": "3-6d1e0a2", …},
 "paths": {"/api/v1/me": {"get": {"summary": "Who the caller is, …", …}}, …}}
```

### `POST /api/v1/opens/{app}`

The caller opened the app. The framework posts it once each time an app's
page loads; it is counted per account and day, and the admins read it as
[usage](admin-apps.md).

| Parameter | In | |
|---|---|---|
| `app` | path | the app's id |

```http
POST /api/v1/opens/recipes
X-Yen-App: recipes

200
{"app": "recipes", "opened": true}
```

### `GET /api/v1/settings/{app}`

The caller's own settings for the app — what they chose in it, kept with
them on every device. `{}` until they choose something. A page reads it as
`app.settings`.

| Parameter | In | |
|---|---|---|
| `app` | path | the app's id |

```http
GET /api/v1/settings/meals

200
{"app": "meals", "settings": {"units": "metric", "week_starts": "monday"}}
```

### `PUT /api/v1/settings/{app}`

Replace the caller's settings for the app: the body is the whole object, up
to 16 KB.

| Parameter | In | |
|---|---|---|
| `app` | path | the app's id |

| Status | Code | When |
|---|---|---|
| 400 | `bad_settings` | the body is not an object, or is too large |

```http
PUT /api/v1/settings/meals
X-Yen-App: meals

{"units": "cups", "week_starts": "monday"}

200
{"app": "meals", "settings": {"units": "cups", "week_starts": "monday"}}
```
