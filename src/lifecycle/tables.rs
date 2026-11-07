//! Every table that holds something of an app's, by its `app` column: what
//! a rename moves and a purge removes (§20.1). A table that gains an `app`
//! column is added here, or a test fails.

/// The tables keyed by app, in the order a purge deletes from them.
pub const APP_TABLES: &[&str] = &[
    "docs_fts",
    "docs",
    "shares",
    "reshapes",
    "job_runs",
    "job_once",
    "jobs",
    "files",
    "ai_calls",
    "secrets",
    "hook_tokens",
    "settings",
    "feedback",
    "app_opens",
    "app_shares",
    "app_access",
    "inbox",
    "mutes",
];

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::store::Store;

    /// **Every table with an `app` column is listed** — but the archive and
    /// the renames, which are about apps rather than of one.
    #[test]
    fn every_table_of_an_app_is_listed() {
        let store = Store::in_memory().unwrap();
        let conn = store.conn();
        let mut stmt = conn
            .prepare(
                "SELECT m.name FROM sqlite_master m, pragma_table_info(m.name) p \
                 WHERE m.type = 'table' AND p.name = 'app'",
            )
            .unwrap();
        let found: BTreeSet<String> = stmt
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        let mut listed: BTreeSet<String> = APP_TABLES.iter().map(|t| t.to_string()).collect();
        listed.insert("archived_apps".into());
        assert_eq!(found, listed);
    }
}
