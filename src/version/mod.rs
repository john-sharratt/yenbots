//! What this binary says of its build: `--build-info`'s answer, one line of
//! JSON in the shape every binary keep supervises gives
//! (`candle/cluster/src/version/info.rs`), so keep can judge it before a
//! trial — and refuses a build with local changes.

/// Run by `build.rs`; compiled here only so its tests run.
#[cfg(test)]
mod compute;

use std::ffi::OsString;

use serde::Serialize;

/// The one argument that asks for the build.
pub const BUILD_INFO_FLAG: &str = "--build-info";

/// This binary's kind, as keep names it.
pub const KIND: &str = "yenbot";

/// `--build-info`'s answer, fields in order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BuildInfo {
    pub kind: String,
    pub count: u32,
    pub hash: String,
    pub local: String,
    pub platform: String,
    /// Yen Bots opens no substrate, decodes no record and runs no model.
    pub log_format: Option<u32>,
    pub record_tags: Vec<u8>,
    pub model: Option<String>,
}

impl BuildInfo {
    /// This binary's.
    pub fn this() -> BuildInfo {
        BuildInfo {
            kind: KIND.into(),
            count: env!("YENBOT_BUILD_COUNT")
                .parse()
                .expect("build.rs stamps a number"),
            hash: env!("YENBOT_BUILD_HASH").into(),
            local: env!("YENBOT_BUILD_LOCAL").into(),
            platform: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
            log_format: None,
            record_tags: Vec::new(),
            model: None,
        }
    }

    /// `count-hash`, with `+local` when it is a local build.
    pub fn label(&self) -> String {
        if self.local.is_empty() {
            format!("{}-{}", self.count, self.hash)
        } else {
            format!("{}-{}+{}", self.count, self.hash, self.local)
        }
    }

    pub fn encode(&self) -> String {
        serde_json::to_string(self).expect("a build's answer always serializes")
    }
}

/// Whether `args` — the process's, without its name — are exactly the flag.
pub fn asks_for_build(args: &[OsString]) -> bool {
    matches!(args, [only] if only == BUILD_INFO_FLAG)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The answer is the cluster's shape exactly**, so keep reads it.
    #[test]
    fn the_answer_is_the_clusters_shape() {
        let info = BuildInfo {
            kind: KIND.into(),
            count: 12,
            hash: "abc1234".into(),
            local: String::new(),
            platform: "windows-x86_64".into(),
            log_format: None,
            record_tags: Vec::new(),
            model: None,
        };
        assert_eq!(
            info.encode(),
            r#"{"kind":"yenbot","count":12,"hash":"abc1234","local":"","platform":"windows-x86_64","log_format":null,"record_tags":[],"model":null}"#
        );
        assert_eq!(info.label(), "12-abc1234");
        let local = BuildInfo {
            local: "ffee".into(),
            ..info
        };
        assert_eq!(local.label(), "12-abc1234+ffee");
    }

    #[test]
    fn only_the_flag_alone_asks() {
        let args = |a: &[&str]| a.iter().map(OsString::from).collect::<Vec<_>>();
        assert!(asks_for_build(&args(&["--build-info"])));
        assert!(!asks_for_build(&args(&[])));
        assert!(!asks_for_build(&args(&["--build-info", "--config", "x"])));
    }

    #[test]
    fn this_binary_names_its_platform() {
        let me = BuildInfo::this();
        assert_eq!(me.kind, "yenbot");
        assert!(me.platform.contains('-'));
    }
}
