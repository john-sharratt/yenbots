# Yen Bot

**Made by Zen.** Yen Bot ([yenbot.com.au](https://yenbot.com.au)) is a site of
small apps that Yen builds by talking to Claude: she says what she wants, and
the app is written, checked and live a moment later.

One Rust binary, `yenbot`, serves the whole site — the landing page, every
app, the API, the store, the art library — on loopback behind the Tokera
gateway on BlackWorld, supervised by keep. The design is
`docs/yenbot_app_host.md` in the candle repository; this repository follows it.

## What is where

| Path | What |
|---|---|
| `src/` | the platform, one concern per file: serving, accounts, the store, the API, the check and the gate, the art library |
| `web/` | the platform's web files, embedded in the binary: the landing page, the framework (`/_/yen.js`), the design system and its catalogue (`/_design`), the library page (`/_library`) |
| `apps/` | one folder per app, served from the live checkout and live on push |
| `library/` | the art library: pictures and `catalogue.yaml` |
| `templates/` | working apps a new app starts from |
| `docs/` | the manual — start at `docs/index.md`; the API reference is `docs/api/` |
| `tests/` | the framework's JavaScript tests, run inside `cargo test` |
| `yenbot.yaml` | the service's config |
| `AGENTS.md` | the rules and map every session reads first |

## How a change goes live

- **An app's files, a picture, a doc**: a commit to `main`. Before a change
  made through Zen lands, the site itself checks it (the gate, `docs/check.md`)
  and refuses it with the file, the line and what to do instead. On a push,
  the site fast-forwards its live checkout and open pages reload.
- **The platform** (`src/`, `web/`, `Cargo.toml`): a new binary, released
  through keep — `docs/running.md`.

## Building and testing

```text
cargo build --release
cargo test --release
cargo fmt --all -- --check
cargo clippy --release --all-targets -- -D warnings
```

All four are clean before every commit. `yenbot --build-info` prints what keep
checks before a trial; `yenbot --config yenbot.yaml` serves, and prints
`INFO serving` once it listens.

## Licence

Private; all rights reserved — see `LICENSE`.
