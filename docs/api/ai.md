# AI and pictures

The model and the image engine, through Zen (`candle/docs/yenbot_app_host.md`
§14; for a page, `app.ai`; for a workflow, `ctx.ai` — `docs/ai.md`). Only an
app whose `app.yaml` says `ai: true` may call them, and only for an active
account that may open it. The site calls Zen itself: no browser ever holds a
credential for it. Every call is kept, with who asked, what, and how long it
took, and the admin page shows what each app has used. There is no budget.

The app is the one the request names: a page's call carries `X-Yen-App`, as
every framework call does; a token's call says `?app=`.

The model reads text only: an app cannot ask it about a photo.

## Routes

### `POST /api/v1/ai/chat`

A chat with the model: the app's own system prompt and messages, as given —
none of Zen's own instructions. Asked for a `schema`, the model answers as
JSON alone, which is read whole and held to the schema's `type`,
`properties`, `required`, `items` and `enum`; an answer that does not fit is
asked for once more, told why. Asked to `stream`, the answer arrives as
server-sent events: `delta` `{content}` for each piece as it is written, then
`done` `{content, json}` — or `error` `{error, detail}`. A schema's answer is
not streamed.

| Field | In | |
|---|---|---|
| `app` | query | the app asking, for a token's call; a page's call names it in `X-Yen-App` |
| `messages` | body | `{role: user\|assistant, content}`, the person's last |
| `system` | body | how the model should answer: the app's own instructions |
| `max_tokens` | body | the longest answer, up to 4000; 1000 when absent |
| `schema` | body | a JSON Schema the answer must match: it is then answered as JSON, read whole |
| `stream` | body | send the answer as server-sent events as it is written |

| Status | Code | When |
|---|---|---|
| 400 | `no_app` | no `X-Yen-App` and no `?app=` |
| 404 | `no_app` | no such app, or none the caller may open |
| 422 | `refused` | the app does not say `ai: true`; there is no message, or the last is not the person's; the model failed; an answer did not fit its schema twice |

```http
POST /api/v1/ai/chat
X-Yen-App: meals

{"system": "You suggest one dinner, in a sentence.", "messages": [{"role": "user", "content": "Something with chicken?"}]}

200
{"content": "Lemongrass chicken with rice and a cucumber salad.", "json": null,
 "usage": {"tokens_in": 31, "tokens_out": 12}}
```

### `POST /api/v1/ai/image`

Draw a picture — or, with `from`, change one of the app's pictures, the
drawing kept close to it. The picture is kept as a file of the app, owned by
the caller ([files](files.md)), and answered with its addresses and the seed
that drew it, which draws the same again. A picture takes from half a minute
to a few minutes; asked to `stream`, its progress arrives as server-sent
events — `progress` `{event: loading|step|waiting, done?, total?}` — then
`done` `{file, seed}`, or `error`.

| Field | In | |
|---|---|---|
| `app` | query | the app asking, for a token's call; a page's call names it in `X-Yen-App` |
| `prompt` | body | what to draw |
| `width` | body | 64 to 1024, a multiple of 16; 1024 when absent |
| `height` | body | 64 to 1024, a multiple of 16; 1024 when absent |
| `seed` | body | the seed, to draw the same again |
| `from` | body | one of the app's pictures, by file id, to change rather than draw anew |
| `hold` | body | how closely a change keeps to `from`, 0 to 0.95; 0.5 when absent |
| `stream` | body | send its progress as server-sent events |

| Status | Code | When |
|---|---|---|
| 422 | `refused` | the app does not say `ai: true`; no prompt; a side out of range; `from` is not one of its pictures; the engine declined or failed |

```http
POST /api/v1/ai/image
X-Yen-App: recipes

{"prompt": "A bowl of phở, soft flat vector, on a cream ground", "width": 768, "height": 512}

200
{"file": {"id": "19a61b2c4e0000004a1", "name": "A bowl of phở, soft flat vector, on a cream ground.png",
          "type": "image/png", "bytes": 412093, "width": 768, "height": 512,
          "url": "/api/v1/files/recipes/19a61b2c4e0000004a1",
          "thumb": "/api/v1/files/recipes/19a61b2c4e0000004a1/thumb", "created_ms": 1794050000000},
 "seed": 2180557391}
```

### `POST /api/v1/ai/ask`

A question about one of the app's collections — "which of my recipes use
chicken?" — answered from its records alone: the collection's searchable
fields (`search:` in `app.yaml`) are searched for the question's words, as
the caller may see them; the best dozen go to the model with their ids; it
answers with the ids it drew on, each one of those it was shown, so the app
can link to them.

| Field | In | |
|---|---|---|
| `app` | query | the app asking, for a token's call; a page's call names it in `X-Yen-App` |
| `question` | body | the question, as the person asked it |
| `from` | body | the collection to answer from; it names fields to search |

| Status | Code | When |
|---|---|---|
| 422 | `refused` | the app does not say `ai: true`; no such collection, or it names no fields to search |

```http
POST /api/v1/ai/ask
X-Yen-App: recipes

{"question": "Which of my recipes use chicken?", "from": "recipes"}

200
{"answer": "Two: chicken phở and lemongrass chicken.", "ids": ["19a5f1aa000000019f", "19a5f2b1000000021c"], "read": 3}
```
