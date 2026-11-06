//! `yenbot.yaml`: where the site listens, where it keeps its data and what it
//! serves, who the admins are, and how a new account becomes active (the
//! design: `candle/docs/yenbot_app_host.md` §3, §12).

use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

/// How a new account becomes active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Registration {
    /// A new account waits for an admin to approve it.
    #[default]
    Approve,
    /// Every account is active at once.
    Open,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// The site's name, as the pages show it.
    #[serde(default = "d_site_name")]
    pub site_name: String,
    /// Where it listens: loopback, behind the gateway.
    #[serde(default = "d_bind")]
    pub bind: SocketAddr,
    /// The store, its backups and its files: `<data_dir>/yenbot.db`.
    pub data_dir: PathBuf,
    /// The live checkout of the repository: the apps, the art library and the
    /// platform's web files are read from it.
    pub live_dir: PathBuf,
    /// The admins, by Google address.
    #[serde(default)]
    pub admins: Vec<String>,
    #[serde(default)]
    pub registration: Registration,
    /// The gateway's bus, where the site hears of every push to its
    /// repository (`repo/yenbot`) and fast-forwards its live checkout.
    pub bus: Bus,
}

/// The gateway's bus: its WebSocket, and yenbot's service key for it.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Bus {
    /// `ws://127.0.0.1/gateway/events` on BlackWorld.
    pub url: String,
    /// The service key's file, 64 hex digits, resolved against the config.
    pub key_file: PathBuf,
}

fn d_site_name() -> String {
    "Yen Bots".into()
}

fn d_bind() -> SocketAddr {
    "127.0.0.1:8096".parse().expect("a literal address")
}

impl Config {
    /// The config in `path`, its relative directories resolved against the
    /// file's own.
    pub fn load(path: &Path) -> Result<Config> {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        Config::from_yaml(&text, path.parent().unwrap_or(Path::new(".")))
            .with_context(|| format!("in {}", path.display()))
    }

    pub fn from_yaml(text: &str, base: &Path) -> Result<Config> {
        let mut cfg: Config = serde_yaml::from_str(text).context("parsing YAML")?;
        for dir in [&mut cfg.data_dir, &mut cfg.live_dir, &mut cfg.bus.key_file] {
            if dir.is_relative() {
                *dir = base.join(&*dir);
            }
        }
        if cfg.admins.is_empty() {
            bail!("`admins` names nobody: someone must be able to approve accounts");
        }
        if !cfg.bind.ip().is_loopback() {
            bail!(
                "`bind` is {}: yenbot listens on loopback only, behind the gateway",
                cfg.bind
            );
        }
        Ok(cfg)
    }

    /// Whether `email` is an admin's.
    pub fn is_admin(&self, email: &str) -> bool {
        let email = email.trim();
        !email.is_empty()
            && self
                .admins
                .iter()
                .any(|a| a.trim().eq_ignore_ascii_case(email))
    }

    /// The store's file.
    pub fn db_path(&self) -> PathBuf {
        self.data_dir.join("yenbot.db")
    }

    /// The nightly backups' folder.
    pub fn backups_dir(&self) -> PathBuf {
        self.data_dir.join("backups")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OK: &str = "data_dir: data\nlive_dir: live\nadmins: [Yen@Example.com]\n\
                      bus: {url: \"ws://127.0.0.1/gateway/events\", key_file: secrets/gateway.key}\n";

    #[test]
    fn a_config_resolves_its_directories_and_defaults() {
        let c = Config::from_yaml(OK, Path::new("/srv/yenbot")).unwrap();
        assert_eq!(c.site_name, "Yen Bots");
        assert_eq!(c.bind, "127.0.0.1:8096".parse().unwrap());
        assert_eq!(c.data_dir, Path::new("/srv/yenbot").join("data"));
        assert_eq!(
            c.db_path(),
            Path::new("/srv/yenbot").join("data").join("yenbot.db")
        );
        assert_eq!(c.registration, Registration::Approve);
        assert_eq!(
            c.bus.key_file,
            Path::new("/srv/yenbot").join("secrets/gateway.key")
        );
        assert!(c.is_admin("yen@example.com "));
        assert!(!c.is_admin("someone@example.com"));
        assert!(!c.is_admin(""));
    }

    /// **The repository's own config is one the service runs with.**
    #[test]
    fn the_shipped_config_runs() {
        let c = Config::from_yaml(include_str!("../yenbot.yaml"), Path::new(".")).unwrap();
        assert_eq!(c.bind, "127.0.0.1:8096".parse().unwrap());
        assert_eq!(c.admins.len(), 2);
        assert_eq!(c.registration, Registration::Approve);
    }

    #[test]
    fn a_config_that_cannot_be_run_is_refused() {
        let bus = "bus: {url: u, key_file: k}\n";
        for bad in [
            format!("data_dir: d\nlive_dir: l\n{bus}"),
            format!("data_dir: d\nlive_dir: l\nadmins: [a@b.c]\nbind: \"0.0.0.0:8096\"\n{bus}"),
            format!("data_dir: d\nlive_dir: l\nadmins: [a@b.c]\ntypo: 1\n{bus}"),
            format!("data_dir: d\nlive_dir: l\nadmins: [a@b.c]\nregistration: maybe\n{bus}"),
            "data_dir: d\nlive_dir: l\nadmins: [a@b.c]\n".to_string(),
        ] {
            assert!(Config::from_yaml(&bad, Path::new(".")).is_err(), "{bad}");
        }
    }
}
