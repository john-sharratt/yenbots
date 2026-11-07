//! Workflows (`candle/docs/yenbot_app_host.md` §10): an app's exported async
//! functions in `workflows/`, run in QuickJS compiled into the binary — a
//! file published by a push runs on the next call, with no build.
//!
//! | Module | |
//! |---|---|
//! | [`loader`] | what a workflow may import: the app's other workflow files |
//! | [`sources`] | an app's workflow files, as it is served |
//! | [`engine`] | one run: its own runtime, a memory ceiling, a deadline |
//! | [`host`] | the platform's side of `ctx` |
//! | [`run`] | a workflow called with `ctx` and its arguments, as its caller |
//!
//! `ctx.js` is the `ctx` a workflow is handed.

pub mod engine;
pub mod host;
pub mod loader;
pub mod run;
pub mod sources;
