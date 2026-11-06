//! What every request is served with: the config, the store and the logs.

use std::sync::Arc;

use crate::config::Config;
use crate::logs::Logs;
use crate::store::Store;
use crate::version::BuildInfo;

pub struct App {
    pub cfg: Config,
    pub store: Arc<Store>,
    pub logs: Logs,
    pub build: BuildInfo,
    pub started_ms: i64,
}
