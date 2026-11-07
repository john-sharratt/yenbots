//! `workflows`: every file in an app's `workflows/` is a workflow the
//! platform can run (§10.1) — it compiles in the engine that runs it, and its
//! default export is a function:
//!
//! ```js
//! export default async function remind(ctx, { record }) { … }
//! ```
//!
//! Each is compiled and its top level run in a QuickJS of its own, with no
//! `ctx` and nothing to reach: a workflow may import the app's other
//! workflow files, and nothing else. The top level runs under a memory
//! ceiling and an instruction budget counted in the engine's interrupt
//! checks, not in time, so the answer is the same on every machine.

use std::collections::BTreeMap;
use std::sync::Arc;

use rquickjs::{CatchResultExt, CaughtError, Context, Module, Runtime, Value};

use crate::check::files::Files;
use crate::check::Refusal;
use crate::workflows::loader::{self, Sources};

use super::app_dir;

pub const RULE: &str = "workflows";

/// The memory a workflow's top level may take.
const MEMORY: usize = 32 << 20;

/// How many of the engine's interrupt checks a top level may run through.
const BUDGET: u32 = 2_000;

/// What went wrong, and where, as the engine tells it.
fn said(caught: CaughtError, path: &str) -> (Option<usize>, String) {
    match loader::said(caught, path) {
        (_, None) => (
            None,
            "runs too long as it loads: a workflow's top level defines its function and does no work".to_string(),
        ),
        (line, Some(message)) => (line, message),
    }
}

/// Why the workflow at `path` cannot be run, if it cannot.
fn compile(workflows: &Sources, path: &str) -> Option<(Option<usize>, String)> {
    let rt = match Runtime::new() {
        Ok(rt) => rt,
        Err(e) => return Some((None, format!("the engine did not start: {e}"))),
    };
    rt.set_memory_limit(MEMORY);
    let mut checks = 0u32;
    rt.set_interrupt_handler(Some(Box::new(move || {
        checks += 1;
        checks > BUDGET
    })));
    rt.set_loader(workflows.clone(), workflows.clone());
    let ctx = match Context::full(&rt) {
        Ok(ctx) => ctx,
        Err(e) => return Some((None, format!("the engine did not start: {e}"))),
    };
    let source = workflows.files.get(path).cloned().unwrap_or_default();
    ctx.with(|ctx| {
        let declared = match Module::declare(ctx.clone(), path, source).catch(&ctx) {
            Ok(m) => m,
            Err(e) => {
                let (line, message) = said(e, path);
                return Some((line, format!("does not compile: {message}")));
            }
        };
        let module = match declared.eval().catch(&ctx) {
            Ok((module, promise)) => match promise.finish::<()>().catch(&ctx) {
                Ok(()) => module,
                Err(e) => {
                    let (line, message) = said(e, path);
                    return Some((line, format!("does not load: {message}")));
                }
            },
            Err(e) => {
                let (line, message) = said(e, path);
                return Some((line, format!("does not load: {message}")));
            }
        };
        let default: Value = module.get("default").unwrap_or_else(|_| Value::new_undefined(ctx.clone()));
        if default.is_function() {
            None
        } else {
            Some((
                None,
                "exports no function: a workflow is `export default async function name(ctx, args) { … }`".to_string(),
            ))
        }
    })
}

pub fn check(files: &Files, id: &str, out: &mut Vec<Refusal>) {
    let dir = format!("{}workflows/", app_dir(id));
    let sources: BTreeMap<String, String> = files
        .texts_under(&dir)
        .filter(|(p, _)| p.ends_with(".js"))
        .map(|(p, t)| (p.to_string(), t.to_string()))
        .collect();
    let workflows = Sources {
        dir,
        files: Arc::new(sources),
    };
    for path in workflows.files.keys() {
        if let Some((line, message)) = compile(&workflows, path) {
            out.push(Refusal::new(RULE, path.clone(), line, message));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(pairs: &[(&str, &str)]) -> Vec<String> {
        let mut f = Files::default();
        for (p, t) in pairs {
            f.add(p, t.as_bytes().to_vec());
        }
        let mut out = Vec::new();
        check(&f, "notes", &mut out);
        out.iter().map(Refusal::to_string).collect()
    }

    #[test]
    fn a_workflow_that_compiles_and_exports_its_function_passes() {
        assert_eq!(
            run(&[
                ("apps/notes/workflows/remind.js", "import { title } from './shared.js';\nexport default async function remind(ctx, { record }) { return title(record); }"),
                ("apps/notes/workflows/shared.js", "export function title(r) { return r.title; }\nexport default (ctx) => 1;"),
            ]),
            Vec::<String>::new()
        );
    }

    /// **A workflow that does not compile, exports no function, reaches
    /// outside its folder or does work as it loads is refused**, with the
    /// engine's own words and the line.
    #[test]
    fn a_workflow_that_cannot_run_is_refused() {
        let told = run(&[
            (
                "apps/notes/workflows/broken.js",
                "export default async function b(ctx) {\n  return (;\n}",
            ),
            ("apps/notes/workflows/empty.js", "export const x = 1;"),
            (
                "apps/notes/workflows/reach.js",
                "import x from '/_/yen.js';\nexport default async function r() {}",
            ),
            (
                "apps/notes/workflows/spin.js",
                "while (true) {}\nexport default async function s() {}",
            ),
        ]);
        assert_eq!(told.len(), 4, "{told:#?}");
        assert!(
            told[0].starts_with("apps/notes/workflows/broken.js:2 does not compile: "),
            "{}",
            told[0]
        );
        assert_eq!(
            told[1],
            "apps/notes/workflows/empty.js exports no function: a workflow is `export default async function name(ctx, args) { … }`"
        );
        assert!(
            told[2].starts_with("apps/notes/workflows/reach.js does not compile: "),
            "{}",
            told[2]
        );
        assert!(
            told[2].contains("a workflow imports only the app's other workflow files"),
            "{}",
            told[2]
        );
        assert_eq!(
            told[3],
            "apps/notes/workflows/spin.js does not load: runs too long as it loads: a workflow's top level defines its function and does no work"
        );
    }
}
