# API tokens

How a Zen task acts on the site (`candle/docs/yenbot_app_host.md` §6.3). An
admin makes a token for their own account; its secret is answered once and
the site keeps only its SHA-256. Save it at once as the Zen credential
`yenbot`, and a task names the credential rather than ever seeing the
secret:

```text
http_session_open {base_url: "https://yenbot.com.au", credential_name: "yenbot"}
http_request {path: "/api/v1/admin/accounts"}
```

A request with `Authorization: Bearer <secret>` acts as the token's account,
admin routes included, and needs no `X-Yen-App`. Every route here answers 403
`forbidden` to a caller who is not an admin.

## Routes

### `GET /api/v1/admin/tokens`

The tokens, oldest first: name, account, when made and when they expire —
never a secret.

```http
GET /api/v1/admin/tokens

200
{"tokens": [{"name": "zen", "account": "1093…", "created_ms": 1793959331042, "expires_ms": 1825495331042}]}
```

### `POST /api/v1/admin/tokens`

Make a token for the caller's account.

| Field | In | |
|---|---|---|
| `name` | body | what it is for; unique |
| `days` | body | how long it lasts, 1 to 366; 365 when absent |

| Status | Code | When |
|---|---|---|
| 400 | `refused` | no name, a name already used, or a life outside 1 to 366 days |

```http
POST /api/v1/admin/tokens

{"name": "zen", "days": 90}

200
{"name": "zen", "secret": "3f9c0e…(64 hex digits)", "note": "shown once: save it as a Zen credential now"}
```

### `DELETE /api/v1/admin/tokens/{name}`

Revoke a token: its next request is nobody's.

| Parameter | In | |
|---|---|---|
| `name` | path | the token's name |

```http
DELETE /api/v1/admin/tokens/zen

200
{"revoked": "zen"}
```
