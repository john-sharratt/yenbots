//! What every request is served with: the config, the store, the logs, the
//! apps, the art library, the open pages' reload signal, the records'
//! change feed, the scheduler's wake and the push sender.

use std::sync::Arc;

use crate::apps::Apps;
use crate::config::Config;
use crate::data::changes::Changes;
use crate::jobs::scheduler::Jobs;
use crate::library::Library;
use crate::logs::browser::Repeats;
use crate::logs::Logs;
use crate::notify::push::Pusher;
use crate::notify::vapid::Vapid;
use crate::publish::Reloads;
use crate::store::Store;
use crate::version::BuildInfo;

pub struct App {
    pub cfg: Config,
    pub store: Arc<Store>,
    pub logs: Logs,
    pub apps: Apps,
    pub library: Library,
    pub reloads: Reloads,
    pub changes: Changes,
    pub jobs: Jobs,
    /// The site's push key, and where pushes are handed to be sent.
    pub vapid: Vapid,
    pub pusher: Pusher,
    /// The browsers' errors seen in the last minute, counted.
    pub repeats: Repeats,
    pub build: BuildInfo,
    pub started_ms: i64,
}
