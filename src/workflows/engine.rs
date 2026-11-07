//! One run in the engine (§10.1, §10.3): a QuickJS runtime of its own, with
//! a memory ceiling and a deadline its interrupt handler holds it to — a
//! loop that never ends is stopped and said so — and nothing kept between
//! runs but what a workflow writes to the store.
//!
//! A run evaluates a driver: a small module, named `yen:driver`, that
//! imports the workflow and `yen:ctx`, calls it, and leaves its answer as
//! JSON in a promise the engine waits out.

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;
use std::time::{Duration, Instant};

use rquickjs::{CatchResultExt, CaughtError, Context, Function, Module, Promise, Runtime};
use serde_json::Value;

use super::host::Host;
use super::loader::{said, Sources};

/// The memory a run may take.
pub const MEMORY: usize = 64 << 20;

/// Why a run did not answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    /// The workflow's file and line, when the engine names one.
    pub at: Option<(String, usize)>,
    pub message: String,
}

impl Failure {
    fn new(message: impl Into<String>) -> Failure {
        Failure {
            at: None,
            message: message.into(),
        }
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.at {
            Some((path, line)) => write!(f, "{path}:{line} {}", self.message),
            None => f.write_str(&self.message),
        }
    }
}

/// What a run is asked to do: the driver's source, the workflow file it
/// runs (for naming a failure's line), what `ctx` says of the run, and the
/// arguments, as JSON the driver reads.
pub struct Driver<'a> {
    pub source: String,
    pub path: &'a str,
    pub run: Value,
    pub args: Value,
}

/// Evaluate `driver` with `host` behind `ctx`, within `deadline`; answers
/// the host back, with what the run did, and the driver's answer.
pub fn evaluate(
    host: Host,
    sources: &Sources,
    driver: Driver<'_>,
    deadline: Duration,
) -> (Host, Result<Value, Failure>) {
    let host = Rc::new(RefCell::new(host));
    let answer = run(&host, sources, &driver, deadline);
    let host = Rc::try_unwrap(host)
        .ok()
        .expect("the run's engine is gone, and its hold on the host with it")
        .into_inner();
    (host, answer)
}

fn run(
    host: &Rc<RefCell<Host>>,
    sources: &Sources,
    driver: &Driver<'_>,
    deadline: Duration,
) -> Result<Value, Failure> {
    let rt = Runtime::new().map_err(|e| Failure::new(format!("the engine did not start: {e}")))?;
    rt.set_memory_limit(MEMORY);
    let stop = Instant::now() + deadline;
    rt.set_interrupt_handler(Some(Box::new(move || Instant::now() > stop)));
    rt.set_loader(sources.clone(), sources.clone());
    let ctx =
        Context::full(&rt).map_err(|e| Failure::new(format!("the engine did not start: {e}")))?;
    let late = || {
        Failure::new(format!(
            "ran past its deadline of {} s and was stopped",
            deadline.as_secs()
        ))
    };
    let path = driver.path;
    let answer = ctx.with(|ctx| -> Result<String, Failure> {
        let failed = |caught: CaughtError<'_>| match said(caught, path) {
            (_, None) => late(),
            (line, Some(message)) => Failure {
                at: line.map(|l| (path.to_string(), l)),
                message,
            },
        };
        let to_host = host.clone();
        let call = Function::new(ctx.clone(), move |op: String, args: String| -> String {
            let args: Vec<Value> = serde_json::from_str(&args).unwrap_or_default();
            to_host.borrow_mut().call(&op, &args).to_string()
        })
        .map_err(|e| Failure::new(e.to_string()))?;
        let globals = ctx.globals();
        let set = |k: &str, v: String| globals.set(k, v).map_err(|e| Failure::new(e.to_string()));
        globals
            .set("__yen", call)
            .map_err(|e| Failure::new(e.to_string()))?;
        set("__yen_run", driver.run.to_string())?;
        set("__yen_args", driver.args.to_string())?;
        let loaded = Module::evaluate(ctx.clone(), "yen:driver", driver.source.clone())
            .catch(&ctx)
            .map_err(failed)?;
        loaded.finish::<()>().catch(&ctx).map_err(failed)?;
        let done: Promise = globals
            .get("__yen_done")
            .map_err(|e| Failure::new(e.to_string()))?;
        done.finish::<String>().catch(&ctx).map_err(failed)
    })?;
    serde_json::from_str(&answer)
        .map_err(|e| Failure::new(format!("answered what is not JSON: {e}")))
}
