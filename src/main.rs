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
mod logs;
mod publish;
mod serve;
mod store;
mod version;

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
use logs::browser::Repeats;
use logs::layer::StoreLayer;
use logs::Logs;
use publish::Reloads;
use store::{backups, Store};
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

    backups::start(store.clone(), cfg.backups_dir());
    let key = std::fs::read_to_string(&cfg.bus.key_file)
        .with_context(|| format!("reading the bus key {}", cfg.bus.key_file.display()))?
        .trim()
        .to_string();
    let reloads = Reloads::new();
    let apps = Apps::open(cfg.live_dir.clone());
    {
        // Each move of the live checkout reads the apps again.
        let (apps, mut moved) = (apps.clone(), reloads.subscribe());
        tokio::spawn(async move {
            while moved.recv().await.is_ok() {
                let apps = apps.clone();
                let _ = tokio::task::spawn_blocking(move || apps.reload()).await;
            }
        });
    }
    publish::start(
        cfg.live_dir.clone(),
        cfg.bus.url.clone(),
        key,
        reloads.clone(),
    );
    let build = BuildInfo::this();
    let bind = cfg.bind;
    let app = Arc::new(App {
        cfg,
        store,
        logs,
        apps,
        reloads,
        changes: Changes::new(),
        repeats: Repeats::new(),
        build: build.clone(),
        started_ms: accounts::now_ms(),
    });
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
