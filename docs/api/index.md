# The API reference

Every route is under `https://yenbot.com.au/api/v1`, answers JSON and acts
as the account the request carries: a signed-in browser's, or an API token's
(`Authorization: Bearer <secret>`, made on [admin-tokens](admin-tokens.md)).
The site serves the same description as OpenAPI 3.1 at
`/api/v1/openapi.json`. Both are written from one table,
`src/serve/routes.rs`, and a test fails when a route has no section here or a
section names a parameter or field its route does not take.

| Page | Routes |
|---|---|
| [site](site.md) | who you are, the apps, reload events, the OpenAPI description |
| [data](data.md) | an app's records: query, sum, search, create, read, update, delete, batch, share |
| [library](library.md) | the art library's pictures, by kind, tag and colour |
| [ai](ai.md) | a chat with the model, a picture drawn or changed, a question about a collection |
| [files](files.md) | an app's files: keep one, fetch it and its thumbnail |
| [notify](notify.md) | the push key, this device's subscription, the inbox, muting, feedback |
| [jobs](jobs.md) | an app's jobs, and running a workflow now |
| [admin-jobs](admin-jobs.md) | every job: run now, hold, release; a workflow run as the app |
| [admin-accounts](admin-accounts.md) | accounts, groups, registration |
| [admin-apps](admin-apps.md) | apps' visibility, shares and order; every account's records |
| [admin-integrations](admin-integrations.md) | an app's keys for other services, and its inbound hooks' tokens |
| [admin-tokens](admin-tokens.md) | API tokens |
| [admin-backups](admin-backups.md) | the nightly backups, and restoring from one |

## Errors

An error is `{"error": <code>, "detail": <sentence>}`, with `field` added when
one field of a record is at fault. Every route can answer:

| Status | Code | When |
|---|---|---|
| 401 | `unauthorized` | nobody is signed in and no token is sent |
| 401 | `bad_token` | `/me` only: the token is not one, or has expired |
| 403 | `not_active` | the account is pending or blocked: only `/me` answers it |
| 403 | `no_app_header` | a browser write without `X-Yen-App`: writes come from the site's own pages |
| 403 | `forbidden` | an admin's route, called by someone who is not one |
| 400 | `bad_json` | the body is not JSON |
| 400 | `bad_query` | the query string cannot be read: a number that is not one |
| 500 | `internal` | the site failed; the detail says how, and the site's log has it |

A token's request needs no `X-Yen-App`: the header exists to stop another
site's page writing as a signed-in browser, and a token is not a browser's.
