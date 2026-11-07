//! The platform's own jobs, app `_platform` (§7.4, §9.1): run by the
//! scheduler like any app's, but in Rust rather than a workflow.
//!
//! | Workflow | Rule | Does |
//! |---|---|---|
//! | `backup` | `daily 03:00`, caught up | the store's nightly backup |
//! | `tidy` | `daily 04:00` | drops jobs run once and done a week ago |

use anyhow::{bail, Error, Result};
use jiff::tz::TimeZone;
use rusqlite::Connection;
use serde_json::json;

use super::rule::Rule;
use super::table::{insert, of_source, tidy, NewJob, PLATFORM};
use crate::config::Config;
use crate::files::mirror::mirror;
use crate::store::backups::back_up;
use crate::store::Store;

/// The platform's jobs: workflow, rule, caught up after the site was down.
pub const JOBS: [(&str, &str, bool); 2] = [
    ("backup", "daily 03:00", true),
    ("tidy", "daily 04:00", false),
];

/// How long a job run once is kept after it is done.
const KEPT_MS: i64 = 7 * 86_400_000;

/// Make each of the platform's jobs that is not there.
pub fn sync(conn: &Connection, tz: &TimeZone, now: i64) -> Result<()> {
    for (workflow, every, catch_up) in JOBS {
        let source = format!("schedule:{workflow}");
        if !of_source(conn, PLATFORM, &source)?.is_empty() {
            continue;
        }
        let rule = Rule::parse(every).map_err(Error::msg)?;
        let Some(next) = rule.next(now, tz) else {
            bail!("the platform's `{every}` never comes");
        };
        insert(
            conn,
            &NewJob {
                app: PLATFORM,
                workflow,
                args: &json!({}),
                source: &source,
                rule: Some(every),
                catch_up,
                next_ms: next,
                run_as: None,
            },
            now,
        )?;
    }
    Ok(())
}

/// Run the platform's `workflow`; answers its log.
pub fn run(workflow: &str, store: &Store, cfg: &Config, now: i64) -> Result<Vec<String>> {
    match workflow {
        "backup" => {
            let written = match back_up(store, &cfg.backups_dir(), now, &cfg.zone())? {
                Some(file) => format!("wrote {}", file.display()),
                None => "today's backup was already written".to_string(),
            };
            let copied = mirror(&cfg.files_dir(), &cfg.kept_files_dir())?;
            Ok(vec![written, format!("copied {copied} new files")])
        }
        "tidy" => {
            let gone = tidy(&store.conn(), now - KEPT_MS)?;
            Ok(vec![format!("dropped {gone} jobs done a week ago")])
        }
        other => bail!("the platform has no job `{other}`"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jobs::table::list;

    #[test]
    fn the_platforms_jobs_are_made_once() {
        let store = Store::in_memory().unwrap();
        let conn = store.conn();
        sync(&conn, &TimeZone::UTC, 0).unwrap();
        sync(&conn, &TimeZone::UTC, 0).unwrap();
        let jobs = list(&conn, Some(PLATFORM), None).unwrap();
        assert_eq!(
            jobs.iter()
                .map(|j| (j.workflow.as_str(), j.next_ms, j.catch_up))
                .collect::<Vec<_>>(),
            [
                ("backup", Some(3 * 3_600_000), true),
                ("tidy", Some(4 * 3_600_000), false)
            ]
        );
    }

    #[test]
    fn the_backup_job_backs_up() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = Config::from_yaml(
            "data_dir: d\nlive_dir: l\nadmins: [a@b.c]\ntime_zone: UTC\nbus: {url: u, key_file: k}\n",
            dir.path(),
        )
        .unwrap();
        let store = Store::in_memory().unwrap();
        let log = run("backup", &store, &cfg, 0).unwrap();
        let file = dir
            .path()
            .join("d")
            .join("backups")
            .join("yenbot-1970-01-01.db");
        assert_eq!(
            log,
            [
                format!("wrote {}", file.display()),
                "copied 0 new files".to_string()
            ]
        );
        assert_eq!(
            run("backup", &store, &cfg, 1).unwrap(),
            ["today's backup was already written", "copied 0 new files"]
        );
        assert!(run("nope", &store, &cfg, 0).is_err());
    }
}
