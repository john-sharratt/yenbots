# Backups

The nightly backups and restoring from one (`candle/docs/yenbot_app_host.md`
§7.4). Once a day the site writes the store whole to
`data\backups\yenbot-YYYY-MM-DD.db`; the newest fourteen are kept. A restore
first writes what it replaces to `data\backups\replaced-<ms>.db`, so a
restore of the wrong day is itself undone by restoring that file's contents.
Every route here answers 403 `forbidden` to a caller who is not an admin.

## Routes

### `GET /api/v1/admin/backups`

The backups held, oldest first, with their sizes in bytes.

```http
GET /api/v1/admin/backups

200
{"backups": [{"date": "2026-11-05", "bytes": 245760}, {"date": "2026-11-06", "bytes": 249856}]}
```

### `POST /api/v1/admin/backups/{date}/restore`

**One app**, the usual case — "I deleted the wrong recipes": that app's
records, their shares and their search entries are replaced with the
backup's — all its collections, or those named — in one transaction. The
rest of the site is untouched and keeps serving.

**The whole store**, for a corrupted database: the same route with no body,
or one without `app`. The backup is copied in under the store's lock and
brought up to this build's schema; requests wait the moment it takes.

| Parameter | In | |
|---|---|---|
| `date` | path | the backup's day, `YYYY-MM-DD` |
| `app` | body | the app whose records come back; the whole store when absent |
| `collections` | body | only these of the app's collections |

| Status | Code | When |
|---|---|---|
| 404 | `no_backup` | there is no backup of that day |
| 404 | `no_app` | the live checkout holds no such app |
| 400 | `bad_request` | `collections` without `app` |
| 409 | `refused` | the backup cannot be restored: one from before the site kept records, or from a newer build |

```http
POST /api/v1/admin/backups/2026-11-05/restore

{"app": "recipes", "collections": ["dishes"]}

200
{"restored": "2026-11-05", "app": "recipes", "records": 41, "replaced": "replaced-1793960400000.db"}
```
