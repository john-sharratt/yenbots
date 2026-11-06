//! What every request is served with: the config, the store, the logs, and
//! the open pages' reload signal.

use std::sync::Arc;

use crate::config::Config;
use crate::logs::Logs;
use crate::publish::Reloads;
use crate::store::Store;
use crate::version::BuildInfo;

pub struct App {
    pub cfg: Config,
    pub store: Arc<Store>,
    pub logs: Logs,
    pub reloads: Reloads,
    pub build: BuildInfo,
    pub started_ms: i64,
}
