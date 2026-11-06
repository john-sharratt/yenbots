# Accounts, groups and registration

The admins' routes for who uses the site (`candle/docs/yenbot_app_host.md`
§12.2, §12.5). Every one answers 403 `forbidden` to a caller who is not an
admin. The admins are the addresses `yenbot.yaml` names: their accounts are
always active, so none of these blocks or removes one.

## Routes

### `GET /api/v1/admin/accounts`

Every account, oldest first.

```http
GET /api/v1/admin/accounts

200
{"accounts": [{"id": "1093…", "email": "yen@example.com", "name": "Yen", "picture": null, "status": "active",
               "admin": true, "created_ms": 1793959331042, "seen_ms": 1793960000000},
              {"id": "2207…", "email": "mai@example.com", "name": "Mai", "picture": null, "status": "pending",
               "admin": false, "created_ms": 1793960100000, "seen_ms": 1793960100000}]}
```

### `POST /api/v1/admin/accounts/{id}/{action}`

Do one thing to an account:

- **`approve`** makes it active.
- **`refuse`** turns a pending account away: it is removed, and the person
  may sign in and ask again.
- **`block`** keeps it from everything but `/me`.
- **`delete`** removes it with its records in private collections; what it
  wrote in shared collections stays.

| Parameter | In | |
|---|---|---|
| `id` | path | the account's id |
| `action` | path | `approve`, `refuse`, `block` or `delete` |

| Status | Code | When |
|---|---|---|
| 404 | `not_found` | no such account, or `refuse` of one that is not pending |
| 404 | `no_action` | the action is none of the four |
| 409 | `refused` | the account is an admin's |

```http
POST /api/v1/admin/accounts/2207…/approve

200
{"account": "2207…", "done": true}
```

### `GET /api/v1/admin/groups`

The groups, by name, with their members' account ids.

```http
GET /api/v1/admin/groups

200
{"groups": [{"name": "household", "members": ["1093…", "2207…"]}, {"name": "kids", "members": []}]}
```

### `POST /api/v1/admin/groups`

Make a group; making one that exists changes nothing.

| Field | In | |
|---|---|---|
| `name` | body | lowercase letters, digits, `-` and `_` |

```http
POST /api/v1/admin/groups

{"name": "kids"}

200
{"group": "kids"}
```

### `DELETE /api/v1/admin/groups/{name}`

Delete a group. Its members' accounts stay; what was shared with the group is
no longer shared with them.

| Parameter | In | |
|---|---|---|
| `name` | path | the group |

```http
DELETE /api/v1/admin/groups/kids

200
{"deleted": "kids"}
```

### `POST /api/v1/admin/groups/{name}/members`

Put an account in a group.

| Parameter | In | |
|---|---|---|
| `name` | path | the group |
| `account` | body | the account's id |

| Status | Code | When |
|---|---|---|
| 400 | `refused` | there is no such group or account |

```http
POST /api/v1/admin/groups/household/members

{"account": "2207…"}

200
{"group": "household", "added": "2207…"}
```

### `DELETE /api/v1/admin/groups/{name}/members/{account}`

Take an account out of a group.

| Parameter | In | |
|---|---|---|
| `name` | path | the group |
| `account` | path | the account's id |

```http
DELETE /api/v1/admin/groups/household/members/2207…

200
{"group": "household", "removed": "2207…"}
```

### `PUT /api/v1/admin/registration`

How a new account becomes active: `approve` — it waits for an admin — or
`open`, active at once. This replaces what `yenbot.yaml` says, until it is
set again.

| Field | In | |
|---|---|---|
| `registration` | body | `approve` or `open` |

```http
PUT /api/v1/admin/registration

{"registration": "open"}

200
{"registration": "open"}
```
