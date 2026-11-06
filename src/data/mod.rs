//! The apps' records (`candle/docs/yenbot_app_host.md` §7.2, §7.5, §7.6):
//!
//! | Module | |
//! |---|---|
//! | [`validate`] | a write checked against its collection's fields |
//! | [`query`] | the `where` language, compiled to bound comparisons |
//! | [`search`] | the full-text index, diacritics folded |
//! | [`docs`] | the records: create, read, update, delete, pages, sums, search, batches, shares |

pub mod docs;
pub mod query;
pub mod search;
pub mod validate;
