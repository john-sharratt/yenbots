# When something breaks

Look before guessing, fix forward, and say plainly what happened.

| Yen says | First | Then |
|---|---|---|
| "It's blank", "it shows an error" | `log_grep` on `yenbot/<app>` for the browser's error — it names the file and line | fix that line in an edit; ask her to look again |
| "It didn't remind me" | the job's runs: `GET /api/v1/admin/jobs?app=<app>` from a task, or `log_grep` for `workflow` | `docs/notifications.md`, "When one does not arrive" |
| "My change didn't show" | was the edit **live**? `GET /api/v1/admin/apps/<id>/check` | a refused change is fixed and sent again; an app refused on arrival is served at its last good version |
| "The site's not loading" | `cluster_status` for the site's health; `log_grep` on `yenbot` | `service_control` restarts yenbot; say what the log showed |
| "I deleted the wrong thing" | `GET /api/v1/admin/backups` | restore that app from last night (`docs/lifecycle.md`) |
| "Undo that" | the journal's note of what was there; the earlier commit | a new edit putting it back — never a history rewrite |

## The rules

- **Read the log first.** Every browser error, failed call and workflow run
  is a line in the app's log, `yenbot/<app>` (`docs/logs.md`).
- **Fail forward.** A fix is a new change; nothing is rewritten.
- **One change at a time**, then ask Yen to look.
- **Stop and ask** before anything she would not expect: deleting records,
  restoring a backup, making an app public, or a change you cannot explain
  to her in a sentence.
