# Integrations

An app's keys for the services it talks to, and the tokens its inbound hooks
are called with (`candle/docs/yenbot_app_host.md` §19; how an app uses them:
`docs/integrations.md`). The admins' alone. A key is never answered once it
is set — not here, not to a page; a workflow reads it with `ctx.secret`. A
hook's token is answered once, when it is made.

## Routes

### `GET /api/v1/admin/secrets/{app}`

The app's keys, by name, with when each was set.

| Field | In | |
|---|---|---|
| `app` | path | the app's id |

```http
GET /api/v1/admin/secrets/weather

200
{"app": "weather", "secrets": [{"name": "api-key", "set_ms": 1794050000000}]}
```

### `PUT /api/v1/admin/secrets/{app}/{name}`

Set one key, replacing what it was. A hook's signing key is the secret
`hook-<name>`.

| Field | In | |
|---|---|---|
| `app` | path | the app's id |
| `name` | path | the key's name: lowercase letters, digits, `-` and `_` |
| `value` | body | the key, up to 8 KB |

| Status | Code | When |
|---|---|---|
| 400 | `bad_secret` | the name is not one, or the value is empty or too long |
| 404 | `no_app` | there is no such app |

```http
PUT /api/v1/admin/secrets/weather/api-key

{"value": "3f9c…"}

200
{"app": "weather", "name": "api-key", "set": true}
```

### `DELETE /api/v1/admin/secrets/{app}/{name}`

Remove one key.

| Field | In | |
|---|---|---|
| `app` | path | the app's id |
| `name` | path | the key's name |

```http
DELETE /api/v1/admin/secrets/weather/api-key

200
{"app": "weather", "removed": "api-key"}
```

### `GET /api/v1/admin/hooks/{app}`

The app's hooks — each file `workflows/hooks/<name>.js` — with the address a
service calls, when its token was made (`null`: none yet), and the secret
its signatures are keyed by.

| Field | In | |
|---|---|---|
| `app` | path | the app's id |

```http
GET /api/v1/admin/hooks/chores

200
{"app": "chores", "hooks": [{"hook": "done", "url": "/hooks/chores/done",
  "token_made_ms": 1794050000000, "signing_secret": "hook-done"}]}
```

### `POST /api/v1/admin/hooks/{app}/{name}`

Make the hook's token, replacing the one it had — the old one stops working
at once. The token is answered here and never again; the answer's `url` is
the whole address to give the service.

| Field | In | |
|---|---|---|
| `app` | path | the app's id |
| `name` | path | the hook, a file `workflows/hooks/<name>.js` |

| Status | Code | When |
|---|---|---|
| 404 | `no_hook` | the app has no such file |

```http
POST /api/v1/admin/hooks/chores/done

200
{"app": "chores", "hook": "done", "token": "yh_8c1e…",
 "url": "/hooks/chores/done?token=yh_8c1e…",
 "note": "shown once: give it to the service that calls in"}
```

### `DELETE /api/v1/admin/hooks/{app}/{name}`

Revoke the hook's token. A call signed with its `hook-<name>` secret still
runs; remove that secret too to close the hook.

| Field | In | |
|---|---|---|
| `app` | path | the app's id |
| `name` | path | the hook |

```http
DELETE /api/v1/admin/hooks/chores/done

200
{"app": "chores", "revoked": "done"}
```
