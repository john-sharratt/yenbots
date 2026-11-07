//! What every request is served with: the config, the store, the logs, the
//! apps, the art library, the open pages' reload signal, the records'
//! change feed, the scheduler's wake and the push sender.

use std::sync::Arc;

use crate::ai::zen::Zen;
use crate::apps::Apps;
use crate::config::Config;
use crate::data::changes::Changes;
use crate::files::Files;
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
    /// The apps' files.
    pub files: Files,
    pub reloads: Reloads,
    pub changes: Changes,
    pub jobs: Jobs,
    /// The site's push key, and where pushes are handed to be sent.
    pub vapid: Vapid,
    pub pusher: Pusher,
    /// Zen's model and image engine (§14).
    pub zen: Arc<dyn Zen>,
    /// The browsers' errors seen in the last minute, counted.
    pub repeats: Repeats,
    pub build: BuildInfo,
    pub started_ms: i64,
}

#[cfg(test)]
impl App {
    /// A site for tests: `cfg`, `store` and `apps`, a library under the
    /// config's data folder, and a push key of fixed bytes whose pushes go
    /// nowhere.
    pub fn for_tests(cfg: Config, store: Arc<Store>, apps: Apps) -> App {
        use crate::ai::fake::FakeZen;
        use p256::SecretKey;
        App {
            library: Library::new(&cfg.live_dir, cfg.library_cache_dir()),
            files: Files::new(cfg.files_dir()),
            cfg,
            logs: Logs::new(store.clone()),
            apps,
            reloads: Reloads::new(),
            changes: Changes::new(),
            jobs: Jobs::new(),
            vapid: Vapid::from_key(
                SecretKey::from_slice(&[7u8; 32]).expect("a key of fixed bytes"),
            ),
            pusher: Pusher::channel().0,
            zen: Arc::new(FakeZen::default()),
            repeats: Repeats::new(),
            store,
            build: BuildInfo::this(),
            started_ms: 0,
        }
    }
}
