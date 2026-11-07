# The manifest

`apps/<id>/app.yaml` (`candle/docs/yenbot_app_host.md` §5.1): what the app is
called, whether it is ready, what it stores and what it may do. It is the
server's — not served — and read again on every change; the gate refuses one
that does not parse or names what is not there, and the app keeps its last
good version.

```yaml
id: reminders                  # the folder's name, the address (/a/reminders/), where its data lives
name: Reminders                # what the landing page and the shell call it
description: Things to remember, with a ping when they are due.
tile: library/tiles/bell-coral # its picture on the landing page, from the library (docs/library.md)
status: live                   # draft (the admins alone see it) | live
guide: 1                       # the edition of docs/building-an-app.md it was built to
theme:                         # its own look, held to read in light and dark (docs/design.md)
  accent: "#b03a5b"
collections:                   # what it stores (docs/data.md)
  reminders:
    scope: private             # each person's own; `shared` (the default): everyone who opens the app reads them
    version: 1                 # its shape; raised with a migration when records change shape (docs/workflows.md)
    fields:
      title: { type: text, required: true, max: 120 }
      due_ms: { type: time, required: true }
    search: [title]            # what full-text search and app.ai.ask read
    index: [due_ms]            # fields filtered or ordered on often
    roles:                     # a shared collection's readers and writers (docs/access.md)
      read: [everyone]
      write: [everyone]
ai: true                       # it may chat with the AI and draw pictures (docs/ai.md)
notify: true                   # its workflows may send notifications (docs/notifications.md)
fetch: [api.open-meteo.com]    # the hosts its workflows may call (docs/integrations.md)
schedules:                     # its jobs (docs/workflows.md)
  - from: reminders
    at: due_ms
    before: 10m
    run: remind
deadline: 5m                   # how long each workflow run may take: 1s to 10m, a minute when absent
renamed_from: remind-me        # the id it had, once, when it was renamed (docs/lifecycle.md)
```

| Key | | Refused when |
|---|---|---|
| `id` | lowercase letters, digits, `-`, `_`, starting with a letter, at most 48 | it is not the folder's name |
| `name`, `description` | words | `name` is missing |
| `tile` | `library/<kind>/<picture>` | the library's catalogue does not hold it |
| `status` | `draft` or `live` | anything else |
| `guide` | the guide's edition | missing on a new app, or older than the guide on a changed one |
| `theme` | `accent`, `accent_dark`, `radius` and the other named tokens (`docs/design.md`) | a colour too pale to read |
| `collections` | each a name and its fields | a name that is not one, a field named for a record's own (`id`, `owner`, `version`, `created_ms`, `updated_ms`, `pending`), a list without `of:`, a ref without `coll:`, a `search` or `index` naming no field |
| `ai`, `notify` | `true` to allow | — |
| `fetch` | host names, exactly as the address has them | an address, `localhost`, a scheme or a path |
| `schedules` | `every:`, `from:`/`at:`/`before:`, or `when:`, each with `run:` | a rule that does not parse, an `at:` that is not a `time` field, a `run:` with no `workflows/<run>.js` |
| `deadline` | `30s`, `5m`, … up to `10m` | outside 1s to 10m |
| `renamed_from` | the app's earlier id | it is the app's own id |

Whether an app is **public or private** is not here: it is Yen's choice on
the admin page, kept in the store (`docs/access.md`).
