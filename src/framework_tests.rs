//! The framework's own tests (`tests/framework/*.test.js`), run in QuickJS:
//! the modules of `web/_/framework/` that need no browser — records, the
//! router, the API's client, error reports, the offline queue, forms'
//! fields, the event streams and the SDK — imported as a page imports them,
//! `/_/…` naming `web/_/…`.
//!
//! Each test file runs in a runtime of its own; every test it registers
//! must pass, and a file that registers none fails.

use std::path::{Component, Path, PathBuf};

use rquickjs::loader::{ImportAttributes, Resolver, ScriptLoader};
use rquickjs::{CatchResultExt, Context, Ctx, Error, Module, Result, Runtime};
use serde::Deserialize;

/// Resolves a module as the browser would against the site: `/_/…` from
/// `web/_/`, `./` and `../` from the importing module, and the driver's own
/// absolute paths as given.
struct Site {
    root: PathBuf,
}

/// `path` with its `.` and `..` resolved, in forward slashes.
fn normal(path: &Path) -> String {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other),
        }
    }
    out.to_string_lossy().replace('\\', "/")
}

impl Resolver for Site {
    fn resolve<'js>(
        &mut self,
        _ctx: &Ctx<'js>,
        base: &str,
        name: &str,
        _attributes: Option<ImportAttributes<'js>>,
    ) -> Result<String> {
        let path = if let Some(rest) = name.strip_prefix("/_/") {
            self.root.join("web").join("_").join(rest)
        } else if name.starts_with("./") || name.starts_with("../") {
            Path::new(base).parent().unwrap_or(Path::new("")).join(name)
        } else if Path::new(name).is_absolute() {
            PathBuf::from(name)
        } else {
            return Err(Error::new_resolving_message(
                base,
                name,
                "a page imports `/_/…` or a relative path",
            ));
        };
        Ok(normal(&path))
    }
}

#[derive(Deserialize)]
struct Outcome {
    name: String,
    ok: bool,
    #[serde(default)]
    error: String,
}

/// Every test `file` registers, and how it went.
fn run(root: &Path, file: &Path) -> std::result::Result<Vec<Outcome>, String> {
    let rt = Runtime::new().map_err(|e| e.to_string())?;
    let ctx = Context::full(&rt).map_err(|e| e.to_string())?;
    rt.set_loader(
        Site {
            root: root.to_path_buf(),
        },
        ScriptLoader::default().with_extension("mjs"),
    );
    let harness = normal(&root.join("tests/framework/harness.js"));
    let test = normal(file);
    let driver = format!(
        "import {{ run }} from {harness:?};\nimport {test:?};\nglobalThis.report = JSON.stringify(await run());\n"
    );
    ctx.with(|ctx| {
        let done = Module::evaluate(ctx.clone(), "driver.js", driver)
            .catch(&ctx)
            .map_err(|e| e.to_string())?;
        done.finish::<()>().catch(&ctx).map_err(|e| e.to_string())?;
        let report: String = ctx.globals().get("report").map_err(|e| e.to_string())?;
        serde_json::from_str(&report).map_err(|e| e.to_string())
    })
}

/// **Every framework test passes.**
#[test]
fn the_frameworks_tests_pass() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files: Vec<PathBuf> = std::fs::read_dir(root.join("tests/framework"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.to_string_lossy().ends_with(".test.js"))
        .collect();
    files.sort();
    assert!(files.len() >= 8, "the test files are found: {files:?}");
    let mut failures = Vec::new();
    let mut passed = 0;
    for file in &files {
        let shown = file.file_name().unwrap().to_string_lossy().to_string();
        match run(root, file) {
            Ok(outcomes) if outcomes.is_empty() => failures.push(format!("{shown}: no tests")),
            Ok(outcomes) => {
                for o in outcomes {
                    if o.ok {
                        passed += 1;
                    } else {
                        failures.push(format!("{shown} — {}: {}", o.name, o.error));
                    }
                }
            }
            Err(e) => failures.push(format!("{shown}: did not run: {e}")),
        }
    }
    assert!(
        failures.is_empty(),
        "{} failed, {passed} passed:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}
