# Files

An app's files (`candle/docs/yenbot_app_host.md` §6.1, §12.6): a photo for a
record, a PDF, a picture the image engine drew. A file is told by its bytes,
never its name — a picture (PNG, JPEG, WebP, GIF) or a PDF, up to 20 MB — and
a picture gets a thumbnail, re-encoded, which drops anything hidden in it. A
record names a file by its id, in a `file` field. A file is never changed once
kept, so a browser keeps it for a year; each is copied once into the backups'
own folder, and restoring an app brings back the files its records name.

Those who may open the app may fetch its files.

## Routes

### `POST /api/v1/files/{app}`

Keep the request's body — the file's bytes as they are — as one of the app's
files, owned by the caller.

| Parameter | In | |
|---|---|---|
| `app` | path | the app's id |
| `name` | query | what the file is called, as the person named it |

| Status | Code | When |
|---|---|---|
| 404 | `no_app` | no such app, or none the caller may open |
| 422 | `bad_file` | it is not a picture or a PDF, or it is over 20 MB |

```http
POST /api/v1/files/recipes?name=pho.jpg
Content-Type: image/jpeg

<the photo's bytes>

201
{"id": "19a61c0d7a000000512", "name": "pho.jpg", "type": "image/jpeg", "bytes": 2140331, "width": 3024, "height": 4032,
 "url": "/api/v1/files/recipes/19a61c0d7a000000512",
 "thumb": "/api/v1/files/recipes/19a61c0d7a000000512/thumb", "created_ms": 1794051000000}
```

### `GET /api/v1/files/{app}/{id}`

The file, as it was kept, with its type.

| Parameter | In | |
|---|---|---|
| `app` | path | the app's id |
| `id` | path | the file's id |

| Status | Code | When |
|---|---|---|
| 404 | `no_file` | the app has no such file |

```http
GET /api/v1/files/recipes/19a61c0d7a000000512

200
Content-Type: image/jpeg
<the photo's bytes>
```

### `GET /api/v1/files/{app}/{id}/thumb`

A picture's thumbnail: WebP, its longest side 480.

| Parameter | In | |
|---|---|---|
| `app` | path | the app's id |
| `id` | path | the file's id |

| Status | Code | When |
|---|---|---|
| 404 | `no_file` | the app has no such picture |

```http
GET /api/v1/files/recipes/19a61c0d7a000000512/thumb

200
Content-Type: image/webp
<the thumbnail's bytes>
```
