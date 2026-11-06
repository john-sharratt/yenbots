# The site

Who the caller is, what they may open, and the description of this API.

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
           "status": "live", "private": false}]}
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

### `GET /api/v1/openapi.json`

This API as OpenAPI 3.1, written from the same table as this reference; its
`info.version` is the build serving it.

```http
GET /api/v1/openapi.json

200
{"openapi": "3.1.0", "info": {"title": "Yen Bots", "version": "3-6d1e0a2", …},
 "paths": {"/api/v1/me": {"get": {"summary": "Who the caller is, …", …}}, …}}
```
