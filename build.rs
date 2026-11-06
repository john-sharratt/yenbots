//! Stamps this build's version into the binary as `YENBOT_BUILD_COUNT`,
//! `YENBOT_BUILD_HASH` and `YENBOT_BUILD_LOCAL`, which `--build-info` answers
//! (`src/version/`), and reruns when the platform's web files change, which
//! the binary embeds.

#[path = "src/version/compute.rs"]
mod compute;

use std::path::Path;

fn main() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");
    let root = Path::new(&manifest);
    for path in compute::rerun_paths(root) {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    let v = compute::compute(root);
    println!("cargo:rustc-env=YENBOT_BUILD_COUNT={}", v.count);
    println!("cargo:rustc-env=YENBOT_BUILD_HASH={}", v.hash);
    println!("cargo:rustc-env=YENBOT_BUILD_LOCAL={}", v.local);
}
