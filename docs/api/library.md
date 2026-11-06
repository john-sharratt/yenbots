# The art library

The pictures every app draws from (`candle/docs/yenbot_app_host.md` §15):
tiles, illustrations, empty states, backgrounds and stickers, drawn in one
house style (`docs/library.md`). The library is the site's own, like its
framework, so this route needs no account. `/_library` is the same search as
a page.

Each picture is served at each of its kind's sizes as
`/_library/<kind>/<id>-<width>.webp`; the URLs this route answers carry a
`?v=` that changes when a picture is redrawn.

| Kind | Sizes |
|---|---|
| `tiles` | 512×512, 256×256, 128×128 |
| `illustrations` | 1024×576, 512×288 |
| `empty` | 600×450, 300×225 |
| `backgrounds` | 1024×1024, 512×512 |
| `stickers` | 256×256, 128×128, on transparency |

## Routes

### `GET /api/v1/library`

The pictures matching every part asked, in id order; with nothing asked,
the whole library. `colour` is the palette colour a picture's dominant
colour is nearest — read from its saturated pixels, so a coral bell on a
cream ground is `coral`.

| Parameter | In | |
|---|---|---|
| `kind` | query | `tiles`, `illustrations`, `empty`, `backgrounds` or `stickers` |
| `tag` | query | one tag the picture carries |
| `colour` | query | `coral`, `rose`, `amber`, `sage`, `teal`, `sky`, `violet` or `slate` |

| Status | Code | When |
|---|---|---|
| 400 | `bad_kind` | `kind` is not one of the five |
| 400 | `bad_colour` | `colour` is not a palette colour |

```http
GET /api/v1/library?kind=tiles&tag=bell

200
{"pictures": [{"id": "tiles/bell-coral", "kind": "tiles", "tags": ["bell", "reminders"],
               "colour": "#d4573d", "palette": "coral", "tiling": false,
               "sizes": [{"width": 512, "height": 512, "url": "/_library/tiles/bell-coral-512.webp?v=3f9c…"},
                         {"width": 256, "height": 256, "url": "/_library/tiles/bell-coral-256.webp?v=3f9c…"},
                         {"width": 128, "height": 128, "url": "/_library/tiles/bell-coral-128.webp?v=3f9c…"}],
               "prompt": "A flat vector illustration of …", "seed": 48151623}]}
```
