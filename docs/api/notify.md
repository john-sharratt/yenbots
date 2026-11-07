# Notifications

Pushes to the devices an account allowed them on, and the inbox that keeps
every notification (`candle/docs/yenbot_app_host.md` §11; how an app sends
one: `docs/workflows.md`, `ctx.notify`). A notification is written to the
inbox first, then pushed to each of the account's devices — unless the
account muted the app, which still writes to the inbox. A device the push
service reports gone is forgotten. There is no email.

The framework does all of this for a page: `app.notify.enable()` asks the
browser and subscribes the device, the bell in every app's shell and on the
landing page reads the inbox (`docs/framework.md`).

## Routes

### `GET /api/v1/push/key`

The site's push key, base64url: a device subscribes with it as its
`applicationServerKey`. No account is needed.

```http
GET /api/v1/push/key

200
{"key": "BLc4xRzKlKORKWlbdgFaBrrPK3ydWAHo4M0gs0i1oEKgPpWC5cW8OCzVrOQRv-1npXRWk8udnW3oYhIO4475rds"}
```

### `POST /api/v1/push/subscribe`

This device, for the caller: the browser's `PushSubscription`, as
`subscription.toJSON()` gives it. A device subscribing again replaces its
own.

| Field | In | |
|---|---|---|
| `endpoint` | body | the push service's address for this device |
| `keys` | body | `{p256dh, auth}`, as the browser gives them |

| Status | Code | When |
|---|---|---|
| 400 | `bad_endpoint` | the endpoint is not an https address |
| 400 | `bad_request` | the body is not a subscription |

```http
POST /api/v1/push/subscribe

{"endpoint": "https://fcm.googleapis.com/fcm/send/c1KrmpTuRm…",
 "keys": {"p256dh": "BIPUL12DLfytvTajnryr2PRdAgXS3HGKiLqndGcJGabyhHheJYlNGCeXl1dn18gSJ1WAkAPIxr4gK0_dQds4yiI", "auth": "FPssNDTKnInHVndSTdbKFw"}}

200
{"subscribed": true}
```

### `DELETE /api/v1/push/subscribe`

Forget this device: it is sent nothing more.

| Field | In | |
|---|---|---|
| `endpoint` | body | the device's push address |

```http
DELETE /api/v1/push/subscribe

{"endpoint": "https://fcm.googleapis.com/fcm/send/c1KrmpTuRm…"}

200
{"unsubscribed": true}
```

### `GET /api/v1/inbox`

The caller's notifications, newest first; how many are unread in all; and
the apps whose pushes they stopped. The newest five hundred are kept.

| Parameter | In | |
|---|---|---|
| `limit` | query | how many, 1 to 100; 30 when absent |

```http
GET /api/v1/inbox?limit=2

200
{"items": [{"id": "19a60e1a2b0000001c4", "app": "chores", "title": "Mai finished the dishes",
            "body": "Ticked off at 6:10 pm", "open": "/a/chores/", "created_ms": 1794046200000, "read_ms": null}],
 "unread": 1, "muted": []}
```

### `POST /api/v1/inbox/{id}/read`

Mark one notification read — or every one, with `all`. It answers how many
it marked.

| Parameter | In | |
|---|---|---|
| `id` | path | the notification's id, or `all` |

```http
POST /api/v1/inbox/all/read

200
{"read": 3}
```

### `PUT /api/v1/notify/{app}/mute`

Stop the app's pushes to the caller, or start them again. A muted app still
writes to the inbox.

| Parameter | In | |
|---|---|---|
| `app` | path | the app's id |
| `muted` | body | whether the app's pushes are stopped |

| Status | Code | When |
|---|---|---|
| 404 | `no_app` | no such app, or none the caller may open |

```http
PUT /api/v1/notify/chores/mute

{"muted": true}

200
{"app": "chores", "muted": true}
```

### `POST /api/v1/feedback/{app}`

A note from the app's own menu, to the admins: written to each one's inbox,
titled with who sent it and the app, and pushed.

| Parameter | In | |
|---|---|---|
| `app` | path | the app's id |
| `text` | body | what the note says, up to 2,000 characters |

| Status | Code | When |
|---|---|---|
| 404 | `no_app` | no such app, or none the caller may open |
| 400 | `bad_request` | there is no `text` |

```http
POST /api/v1/feedback/chores

{"text": "Can the chores be sorted by who does them?"}

200
{"sent": {"accounts": 2, "devices": 3}}
```
