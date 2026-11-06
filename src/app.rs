//! What every request is served with: the config, the store, the logs, the
//! apps, the art library, the open pages' reload signal and the records'
//! change feed.

use std::sync::Arc;

use crate::apps::Apps;
use crate::config::Config;
use crate::data::changes::Changes;
use crate::library::Library;
use crate::logs::browser::Repeats;
use crate::logs::Logs;
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
    /// The browsers' errors seen in the last minute, counted.
    pub repeats: Repeats,
    pub build: BuildInfo,
    pub started_ms: i64,
}
