# Running Yen Bot

## Where it is

| | |
|---|---|
| Machine | BlackWorld, beside the gateway |
| Service | `yenbot`, supervised by keep (`keep/stable/yenbot.exe`) |
| Listens | `127.0.0.1:8096` — loopback only; the gateway forwards to it |
| Config | `D:\yenbot\live\yenbot.yaml` — this repository's `yenbot.yaml`, as `main` holds it; read at start |
| Library cache | `D:\yenbot\data\library\` — each picture sized once, by content |
| Store | `D:\yenbot\data\yenbot.db` — SQLite, schema migrated on start |
| Live files | `D:\yenbot\live` — a checkout of `origin/main`, never edited by hand; cloned with `git clone -c core.autocrlf=false git@github.com:john-sharratt/yenbots.git D:\yenbot\live` so it serves what was committed, byte for byte |
| Bus key | `D:\yenbot\secrets\gateway.key` — yenbot's service key on the gateway (`cluster.service_keys` in `web/web.yaml`), 64 hex digits |
| Backups | `D:\yenbot\data\backups\yenbot-YYYY-MM-DD.db`, one a day at 03:00 in Yen's time zone and named for her day, the newest 14 — the platform's `backup` job, run on start when it was missed |
| Time zone | `time_zone` in `yenbot.yaml`, `Australia/Sydney`: repeat rules, `ctx.time` and the backups' names read it |

## Publishing

Every push to `main` is announced on the gateway's bus as `repo/yenbot`; the site
fetches and fast-forwards the live checkout at once, and every minute besides. A history
rewrite on origin is refused and logged: the site keeps serving what it had. Each move
reloads the open pages.

## Releasing

A change under `src/`, `web/`, `Cargo.toml` or `build.rs` is a new binary:

1. A Zen task commits it to `main` and pushes.
2. `service_deploy {host: BlackWorld, service: yenbot}` builds it on BlackWorld and has
   keep try it.
3. keep runs the new binary and keeps it once it prints `INFO serving` on stderr, or
   puts the old one back. keep only tries a build of a commit on `origin/main`: the
   binary's `--build-info` must name no local changes.
4. `job_status` on the deploy's handle watches it; `GET /v1/status` answers the build
   it runs.

## Jobs

One scheduler runs every job as it falls due, four at a time (`workflows.md`): the apps'
and the platform's own, `_platform` — the nightly `backup` and the `tidy` that drops
one-off jobs done a week ago. A job left running when the site stopped runs again on
start; one that fell due while it was down runs if it is under an hour late, or its
schedule catches up.

## Health

`GET /v1/status`: `{"state": "ready", "build": "<count>-<hash>", "started_at_ms",
"store": {"schema"}}`.
