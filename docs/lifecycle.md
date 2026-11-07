# Apps over time: export, import, delete, rename, backups

(`candle/docs/yenbot_app_host.md` §7.4, §20.) Every route here is in
`docs/api/`, called from a Zen task (`docs/calling-the-api.md`).

## Export and import

- **Out**: `GET /api/v1/export/<app>` — every collection as JSON — or
  `?format=csv&coll=<coll>`, one collection for a spreadsheet, its accents
  intact. An admin's export holds every account's records.
  (`docs/api/data.md`)
- **In**: `POST /api/v1/import/<app>` with the JSON an export writes, or
  `?format=csv&coll=<coll>` with a spreadsheet's CSV: a header of field
  names, a record a line. Each record is made new and held to its fields;
  one refused stops the import and nothing is kept, the answer naming which
  record and why.

## Deleting an app

Remove its folder (`apps/<id>/`) in a commit. The site archives it: nobody
is served it, its jobs are held, and its records, files and jobs are kept
thirty days — listed on the admin page and at `GET /api/v1/admin/archive` —
then the nightly `purge` removes them. Bringing the folder back (a task
reads it from the commit before it was removed and writes it again) restores
everything as it was. An admin can also archive an app without removing it
(`POST /api/v1/admin/apps/<id>/archive`), and restore it.

## Renaming an app

In one commit — a Zen task — move `apps/<old>/` to `apps/<new>/`, set `id:
<new>`, and add `renamed_from: <old>` to its `app.yaml`. On load the site
moves the old id's records, files, jobs, access, keys and settings to the new
id, once, and `/a/<old>/…` redirects. Rename the page tags too if they carry
the old id.

## Storage

The admin page shows each app's records and files. Files no record names —
a photo whose record was deleted, a picture drawn and not kept — are removed
by the nightly `purge` a week after they were made.

## Backups

Every night at 03:00, Yen's time, the whole store is backed up, and the
apps' files are copied beside it; the newest fourteen are kept.

- `GET /api/v1/admin/backups` lists them.
- `POST /api/v1/admin/backups/<date>/restore {"app": "<id>"}` brings back
  one app's records (`"collections": [...]` for only some) and its files as
  they were that night — every other app untouched, and a backup of what was
  replaced written first.
- Without `app`, the whole store comes back: only when something has gone
  badly wrong, and with Yen's word.

"I deleted the wrong recipes" is a restore of that app from last night's
backup (`docs/api/admin-backups.md`).
