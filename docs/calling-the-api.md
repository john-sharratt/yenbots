# Calling the API

Everything the site stores — records, jobs, usage, accounts — is its API
(`docs/api/`). claude.ai has no HTTP of its own, so it calls the API through
a Zen task on the yenbot repository whose brief says which calls to make
(`candle/docs/yenbot_app_host.md` §6.3). The task's worker opens a session
on the site with the `yenbot` credential — an API token for an admin's
account, held by Zen so no chat ever sees it — and makes the calls; its
result carries the answers.

## The recipe

Every brief tells the worker to:

1. `http_session_open {base_url: "https://yenbot.com.au", credential_name: "yenbot"}`.
2. Make each call with `http_request {path, method, body}`: the path is the
   route's (`/api/v1/admin/jobs`), the body JSON. A token's calls need no
   `X-Yen-App`; a route an app's page calls with it takes `?app=<id>` instead.
3. Check each answer's status: `200`/`201` is done; anything else is
   `{"error", "detail"}`, and the detail says why — report it, don't retry
   blindly.
4. Answer with what was asked, in plain words and with the numbers, not
   the raw JSON.

A task takes a minute or two. Tell Yen it is on its way, wait with
`job_status`, and read her the answer.

## Briefs to copy

**See an app's records** — "Are my recipes actually saved?"

```text
In the yenbot repository, call the site's API: open an http session with
base_url https://yenbot.com.au and credential yenbot, then
GET /api/v1/admin/data/recipes/recipes?limit=200 (and the next pages with
?after= while `after` is set). Answer with how many records there are and
each one's title, newest first.
```

**List the jobs, and run one now** — "Did the reminder go out?", "Run it now
so I can see"

```text
In the yenbot repository, call the site's API (http session, base_url
https://yenbot.com.au, credential yenbot): GET /api/v1/admin/jobs?app=meals.
Answer with each job's workflow, when it runs next and its last run's time
and outcome. Then POST /api/v1/admin/jobs/<the plan-meals job's id>/run and
GET /api/v1/admin/jobs?app=meals again until its newest run is after the
call; answer with how that run went and what it logged.
```

**Approve an account** — "Approve my cousin"

```text
In the yenbot repository, call the site's API (http session, base_url
https://yenbot.com.au, credential yenbot): GET /api/v1/admin/accounts, find
the pending account named <name>, and POST /api/v1/admin/accounts/<its id>/approve.
Answer with the account's name, address and status after.
```

**Make an app the family's** — "Only the family should see the chores app"

```text
In the yenbot repository, call the site's API (http session, base_url
https://yenbot.com.au, credential yenbot): PUT /api/v1/admin/apps/chores/visibility
{"private": true}, then PUT /api/v1/admin/apps/chores/shares
{"who": ["group:household"]}. Answer with both answers.
```

**Read the usage** — "How much is each app being used?"

```text
In the yenbot repository, call the site's API (http session, base_url
https://yenbot.com.au, credential yenbot): GET /api/v1/admin/usage?by=day&since=<four weeks ago, YYYY-MM-DD>
and GET /api/v1/admin/usage?by=account&since=<the same>. Answer per app with
its opens, how many people opened it, on how many days, and its AI calls.
```

**Restore an app's records** — "I deleted the wrong recipes"

```text
In the yenbot repository, call the site's API (http session, base_url
https://yenbot.com.au, credential yenbot): GET /api/v1/admin/backups, then
POST /api/v1/admin/backups/<last night's date>/restore {"app": "recipes"}.
Then GET /api/v1/admin/data/recipes/recipes?limit=200 and answer with how
many recipes there are now and their titles.
```

**Check a write landed** — after a task changed records

```text
… then GET /api/v1/admin/data/<app>/<coll>/<the id> and answer with its
fields and version.
```

**Export to a spreadsheet** — "Put the recipes in a spreadsheet"

```text
… GET /api/v1/export/recipes?format=csv&coll=recipes and answer with the
CSV's first lines and how many rows it has.
```

## The token

The `yenbot` credential is an API token named `zen`, made on the admin page
for an admin's account, lasting a year (`docs/api/admin-tokens.md`). The
site keeps only its hash. When it expires or is revoked, an admin makes a
new one and it is saved in Zen again as `yenbot`.
