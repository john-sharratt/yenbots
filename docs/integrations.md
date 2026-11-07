# Integrations

How an app talks to other services: calling out to one from a workflow, with
a key kept on the admin page, and being called in by one through a hook
(`candle/docs/yenbot_app_host.md` §19). The routes are on
[admin-integrations](api/admin-integrations.md).

Google services — a person's calendar, Drive, tasks — are not here yet.

## Calling out

A workflow calls a service with `ctx.fetch`, as a browser's `fetch` does. Only
a workflow can: an app's pages never call another site (the
Content-Security-Policy refuses it), so a page asks a workflow, which calls
out and keeps what it needs in a collection.

The app lists each host it calls in `app.yaml`, exactly as the address names
it — `api.open-meteo.com`, not `open-meteo.com`, and never an address or
`localhost`:

```yaml
fetch: [api.open-meteo.com]
```

```js
// workflows/weather.js
export default async function weather(ctx) {
  const r = await ctx.fetch("https://api.open-meteo.com/v1/forecast?latitude=-33.87&longitude=151.21&current=temperature_2m");
  if (!r.ok) throw new Error(`the forecast answered ${r.status}`);
  const now = (await r.json()).current.temperature_2m;
  await ctx.data.collection("today").create({ temp: now });
  return now;
}
```

- `ctx.fetch(url, {method, headers, body})`: `https://` only; `GET`, `POST`,
  `PUT` or `DELETE`; a `body` that is not a string is sent as JSON, with
  `Content-Type: application/json` unless the call sets one.
- It answers `{status, ok, headers, text(), json()}` whatever the status —
  a 404 or a 500 is an answer, not an error. Header names are lowercase.
- It throws `forbidden` for a host `fetch:` does not list, a method it does
  not make, or a header the site sets itself (`Host`, `Content-Length`);
  `unreachable` when the service did not answer in 30 seconds. A redirect
  is answered as it is, never followed.
- At most 5 MB of an answer is read.

## Keys

A service's key — an API key, a password — is set on the admin page, or by
a task: `PUT /api/v1/admin/secrets/<app>/<name> {"value": "…"}`. It is never
shown again, never answered to a page, and never written into the
repository. A workflow reads it:

```js
const key = ctx.secret("api-key");   // throws `no_secret` when it is not set
const r = await ctx.fetch("https://api.example.com/v1/x", { headers: { Authorization: `Bearer ${key}` } });
```

Never log a key, return it, or keep it in a record: a workflow's answer and
log are shown to whoever ran it.

## Being called in: hooks

A hook is a workflow another service runs by calling the site — a form
elsewhere, a shortcut on a phone. It is a file `workflows/hooks/<name>.js`,
called at `https://yenbot.com.au/hooks/<app>/<name>` with `GET`, `POST` or
`PUT`. It runs as the app (`ctx.account` is `null`), handed the request:

```js
// workflows/hooks/done.js — a phone shortcut marks a chore done
export default async function done(ctx, req) {
  const id = req.json?.chore ?? req.query.chore;
  await ctx.data.collection("chores").update(id, { done: true });
  return { marked: id };
}
```

| `req.` | |
|---|---|
| `method` | `GET`, `POST` or `PUT` |
| `query` | the query string's values, by name (never the token) |
| `headers` | the request's headers, lowercase — never `authorization` or `cookie` |
| `text` | the body as text |
| `json` | the body read as JSON, or `null` when it is not |

The service gets `{"ok": true, "value": <what the hook returned>}`, or a
500 `{"error": "hook_failed", "detail"}` when it threw.

A hook is reachable without signing in, so every call proves itself, one of
two ways:

- **Its token**, made on the admin page or by a task —
  `POST /api/v1/admin/hooks/<app>/<name>`, answered once — sent as
  `?token=<token>` or `Authorization: Bearer <token>`. Making a new one
  stops the old one.
- **A signature** of the body, for a service that signs what it sends: the
  hex HMAC-SHA256 of the body, keyed by the app secret `hook-<name>`, in
  `X-Yen-Signature: sha256=<hex>` (GitHub's `X-Hub-Signature-256` is read
  too). Set the secret to the key the service signs with.

A call with neither gets 401 `unsigned`, and a line in the app's log says
so — read it with `log_grep` on `yenbot/<app>` when a service says it was
refused.
