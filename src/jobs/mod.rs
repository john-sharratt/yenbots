//! Scheduled jobs (`candle/docs/yenbot_app_host.md` §9): a job is a row in
//! the store — which app, which workflow, with what, when it next runs and
//! how its last runs went — and one scheduler runs them.
//!
//! | Module | |
//! |---|---|
//! | [`span`] | `10m`, `2h`: a manifest's lengths of time |
//! | [`cron`] | a cron line |
//! | [`rule`] | a repeat rule, read in Yen's time zone |
//! | [`clock`] | a time as Yen reads it |
//! | [`table`] | the jobs and their runs |
//! | [`follow`] | the jobs a write makes, moves or ends, in its transaction |
//! | [`sync`] | an app's jobs brought into step with its manifest |
//! | [`platform`] | the platform's own jobs: the backup, the tidy |
//! | [`runner`] | one run, and how it went kept |
//! | [`scheduler`] | the loop that runs what is due, and catches up on start |

pub mod clock;
pub mod cron;
pub mod follow;
pub mod platform;
pub mod rule;
pub mod runner;
pub mod scheduler;
pub mod span;
pub mod sync;
pub mod table;
