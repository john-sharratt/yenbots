# Logs

Everything that happens on the site is written down where the connector reads it.

| Source | What |
|---|---|
| `yenbot` | the platform's own lines: starts, publishes, refusals, migrations, failures |
| `yenbot/<app>` | one app's: browser errors, workflow runs, check refusals |

Lines are kept thirty days, in the store.

## Reading them

On the connector:

- `log_list` — every source, how many lines, the newest line's time.
- `log_read {source: "yenbot/recipes"}` — a source's newest lines.
- `log_grep {grep: "TypeError", source: "yenbot/recipes", since: "2026-11-06T09:00"}` —
  a pattern, in one source or all.

Each line reads `<time UTC> <LEVEL> <source> <target>: <message>`. Quote the lines you
rely on.

## Under the hood

The site answers `GET /v1/logs?grep&level&since&limit&source` and `GET /v1/logs/sources`
to admins; the gateway reaches them for the connector at `/gateway/logs/yenbot/…`.
