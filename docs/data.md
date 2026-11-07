# Data

An app's records (`candle/docs/yenbot_app_host.md` §7): each collection a set
of records whose fields `app.yaml` declares, kept in the site's one store and
held to those fields on every write. A page reads and writes them through a
store (`docs/framework.md`), a workflow through `ctx.data`
(`docs/workflows.md`), a task through the API (`docs/api/data.md`).

## Fields

| `type` | Holds | A form shows |
|---|---|---|
| `text` | words; `long: true` for a paragraph | a line, or a box |
| `number` | a number | a number |
| `bool` | true or false | a checkbox |
| `time` | a moment, in milliseconds | a date and a time |
| `date` | a day, `YYYY-MM-DD` | a date |
| `file` | one of the app's files, by id (`docs/ai.md`) | — the page uploads it |
| `list` | items of `of:` (`text`, `number`, …) | words separated by commas, for `of: text` |
| `object` | a small object of anything | — |
| `ref` | another record's id, in `coll:` | — |

Each may say `required: true`; `min` and `max` (a number's value; the
characters of a text; the items of a list); `default`, given when a new
record leaves it out. A write naming a field the collection does not
declare is refused, with the field named — so a misspelt field is found at
once, not discovered missing later. Text is kept in Unicode's composed form,
so "phở" is always the same bytes.

A record also has its own: `id`, `owner` (the account that made it, or
`app:<id>` for a workflow running as the app), `version` (how many writes it
has had), `created_ms` and `updated_ms`.

## Scope

- **`shared`** (the default): every account that may open the app reads
  them; who may write them is `roles:` (`docs/access.md`).
- **`private`**: each record is its owner's alone, unless they share it —
  their notes, their journal, their reminders.

## Reading

| | Page | Workflow | API |
|---|---|---|---|
| a page of records | `app.collection(name, {where, order})` | `list({where, order, limit, after})` | `GET /data/<app>/<coll>` |
| one | `.get(id)` | `get(id)` | `GET /data/<app>/<coll>/<id>` |
| counts and sums | `.sum({group, of, where})` | `sum({where, group, of})` | `GET …/_sum` |
| search | `.search(words)` | `search(q)` | `GET …/_search?q=` |

**`where`** is clauses joined by ` and `: `field op value`, ops `=` `!=` `<`
`<=` `>` `>=` and `~` (contains) — `minutes <= 30 and kind = soup`,
`due_ms >= 1794000000000`. **`order`** is a field, `-field` for descending;
creation order when absent. **Search** reads the collection's `search:`
fields, matching each word as a prefix, accents ignored, best first.
**Sums** group by one or two fields and give the count, and with `of` the
sum, average, least and greatest of a number field.

## Writing

`add`, `update` (a field set to `null` is removed), `remove`; and from the API
a batch — several writes in one transaction, all or none. A write made
against a version that has moved on is refused as `stale`, and the page
offers "Keep mine" or "Drop it": nobody's change is silently overwritten.

## Changing shape

Adding an optional field is an edit, nothing more. Renaming one, splitting
one, or changing what one means raises the collection's `version` and ships
`workflows/migrate-<coll>-<n>.js` for each step (`docs/workflows.md`); the
site takes every record up while the app is served as it was.

## Out and in

Any app's records go out as JSON, or one collection as CSV for a
spreadsheet, and come back in validated like any write
(`docs/lifecycle.md`).
