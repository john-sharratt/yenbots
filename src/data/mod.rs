//! The apps' records (`candle/docs/yenbot_app_host.md` §7.2, §7.5, §7.6):
//!
//! | Module | |
//! |---|---|
//! | [`validate`] | a write checked against its collection's fields |
//! | [`nfc`] | text normalised to NFC on every write |
//! | [`query`] | the `where` language, compiled to bound comparisons |
//! | [`search`] | the full-text index, diacritics folded |
//! | [`docs`] | the records: create, read, update, delete, pages, sums, search, batches, shares |
//! | [`changes`] | each write told to the open pages that may see it |

pub mod changes;
pub mod csv;
pub mod docs;
pub mod export;
pub mod import;
pub mod nfc;
pub mod query;
pub mod search;
pub mod validate;
