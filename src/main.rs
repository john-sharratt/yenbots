//! `yenbot` — Yen Bots, the app-hosting site at yenbot.com.au: one binary
//! serving the platform, its apps, its API and its store, on loopback behind
//! the Tokera gateway, supervised by keep (`candle/docs/yenbot_app_host.md`).
//!
//! ```text
//! yenbot --build-info          what keep checks before a trial
//! yenbot --config yenbot.yaml  serve; prints `INFO serving` once listening
//! ```

mod accounts;
mod app;
mod apps;
mod check;
mod config;
mod data;
mod gate;
mod git;
mod jobs;
mod library;
mod logs;
mod publish;
mod reshape;
mod serve;
mod store;
mod version;
mod workflows;

#[cfg(test)]
mod framework_tests;

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use app::App;
use apps::Apps;
use config::Config;
use data::changes::Changes;
use jobs::scheduler::{self, sync_all, Jobs};
use library::Library;
use logs::browser::Repeats;
use logs::layer::StoreLayer;
use logs::Logs;
use publish::Reloads;
use store::Store;
use version::{asks_for_build, BuildInfo};

/// The whole line keep's trial waits for on stderr before it keeps a new
/// binary (`candle/cluster/src/upgrade/switch.rs` `PROOF`), printed once the
/// site is listening.
const PROOF: &str = "INFO serving";

/// The config file `args` name: `--config <path>`.
fn config_path(args: &[OsString]) -> Result<PathBuf> {
    match args {
        [flag, path] if flag == "--config" => Ok(PathBuf::from(path)),
        _ => bail!("usage: yenbot --config <yenbot.yaml> | yenbot --build-info"),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    if asks_for_build(&args) {
        println!("{}", BuildInfo::this().encode());
        return Ok(());
    }
    let cfg = Config::load(&config_path(&args)?)?;
    let store = Arc::new(Store::open(&cfg.db_path())?);
    let logs = Logs::new(store.clone());
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .with(StoreLayer::new(logs.clone()))
        .init();

    let key = std::fs::read_to_string(&cfg.bus.key_file)
        .with_context(|| format!("reading the bus key {}", cfg.bus.key_file.display()))?
        .trim()
        .to_string();
    let reloads = Reloads::new();
    let apps = Apps::open(cfg.live_dir.clone(), store.clone());
    let library = Library::new(cfg.live_dir.clone(), cfg.library_cache_dir());
    let build = BuildInfo::this();
    let bind = cfg.bind;
    let live_dir = cfg.live_dir.clone();
    let bus_url = cfg.bus.url.clone();
    let app = Arc::new(App {
        cfg,
        store,
        logs,
        apps,
        library,
        reloads: reloads.clone(),
        changes: Changes::new(),
        jobs: Jobs::new(),
        repeats: Repeats::new(),
        build: build.clone(),
        started_ms: accounts::now_ms(),
    });
    {
        // The library is read once at start, then again — with the apps and
        // their jobs — on each move of the live checkout.
        let (app, mut moved) = (app.clone(), reloads.subscribe());
        tokio::spawn(async move {
            let first = app.library.clone();
            let _ = tokio::task::spawn_blocking(move || first.reload()).await;
            while moved.recv().await.is_ok() {
                let app = app.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    app.apps.reload();
                    app.library.reload();
                    if let Err(e) = sync_all(&app, accounts::now_ms()) {
                        tracing::error!(target: "yenbot::jobs", "the platform's jobs could not be made: {e:#}");
                    }
                    app.jobs.wake();
                })
                .await;
            }
        });
    }
    publish::start(live_dir, bus_url, key, reloads);
    scheduler::start(app.clone());
    reshape::start(app.clone());
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .with_context(|| format!("binding {bind}"))?;
    tracing::info!(build = %build.label(), %bind, "serving");
    eprintln!("{PROOF}");
    axum::serve(listener, serve::router(app)).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_config_is_named_by_one_flag() {
        let args = |a: &[&str]| a.iter().map(OsString::from).collect::<Vec<_>>();
        assert_eq!(
            config_path(&args(&["--config", "yenbot.yaml"])).unwrap(),
            PathBuf::from("yenbot.yaml")
        );
        assert!(config_path(&args(&[])).is_err());
        assert!(config_path(&args(&["yenbot.yaml"])).is_err());
    }
}
