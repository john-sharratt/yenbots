# Accounts, groups and sharing

Who may use the site, and who sees what in it (`candle/docs/yenbot_app_host.md`
§12). The routes are `docs/api/admin-accounts.md` and
`docs/api/admin-apps.md`.

## Accounts

A person signs in with Google; the gateway signs them in and the site
believes it. Their first visit makes their account:

- **Waiting** (`pending`) while `registration` is `approve` — the default:
  they see a page saying Yen has been told, and Yen gets a notification
  with the way to the admin page, where either admin approves or refuses.
- **Active** at once when `registration` is `open`.
- **Blocked**: sees nothing, calls nothing.

The admins are named in `yenbot.yaml` by Google address — Yen and John —
and nowhere else; an admin is always active, sees drafts, the admin page and
every app's jobs. Children sign in with their own Google accounts and are
approved and grouped like anyone.

An account deleted loses its private records; what it shared stays, shown as
by a former member.

## Apps: public or private

Every live app is **public**: every active account sees it on the landing
page and may open it. Yen can make one **private** on the admin page —
then only she and those she shares it with, by name or by group, see it or
any of its data. Nothing is deleted by making an app private. A **draft**
opens for the admins alone, whatever its visibility.

**Ask Yen** before an app leaves draft or becomes public.

## Inside an app

Three levels, applied by the site to every read, write, search, sum, export
and change heard:

- **`scope: private` collections**: each person's records are their own.
- **Roles on a shared collection** — who reads and who writes, `everyone`
  or groups:
  ```yaml
  roles: { read: [everyone], write: [household] }
  ```
  The kids see the chores; only the parents change them.
- **A record shared**: one record of a private collection shared with named
  accounts or groups, to read or to edit — a shopping list for Grandma
  (`PUT /data/<app>/<coll>/<id>/_share`, `docs/api/data.md`).

The framework hides what a person cannot do (`mayAdd`, `mayEdit`), so a
child sees the list without the buttons, rather than buttons that fail.

## Groups

Kept by the admins: `household`, `kids`, `grandparents` — any name, each a
set of accounts. A group's name is written into apps' `roles:` and shares,
so a group is never renamed: make the new one, move its people, then delete
the old.

## What the platform keeps safe

Every app, without asking:

- **A strict content policy**: scripts, styles, fonts and pictures from the
  site only; no inline script, no `eval`.
- **Forgery refused**: a write needs the `X-Yen-App` header the framework
  sends, which another site's page cannot.
- **Markup escaped**: every value is escaped as it is drawn; the gate
  refuses `unsafeHTML` and its kin.
- **Uploads checked by their bytes**, with a size ceiling; pictures are
  re-encoded for their thumbnails.
- **Keys never reach a browser** (`docs/integrations.md`).
- **API tokens** are kept only as their hash, act as one account, expire,
  and are revoked on the admin page.

**What an app must never do:** keep a password, a key or anything secret in
a record or a page; name a person's private details in a public app; work
around a refusal rather than fixing what it names.
