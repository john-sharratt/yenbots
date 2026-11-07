//! What a workflow may import, for both the check that compiles it and the
//! engine that runs it: the app's other workflow files, by a relative path,
//! and nothing else. The engine's own modules (named `yen:…`) import a
//! workflow by its whole path, and `yen:ctx` (`ctx.js`), the `ctx` a run is
//! handed.

use std::collections::BTreeMap;
use std::sync::Arc;

use rquickjs::loader::{ImportAttributes, Loader, Resolver};
use rquickjs::module::Declared;
use rquickjs::{CaughtError, Ctx, Error, Module};

/// The module every workflow run is handed its `ctx` from.
pub const CTX: &str = "yen:ctx";

/// `ctx`, as JavaScript.
const CTX_SOURCE: &str = include_str!("ctx.js");

/// An app's workflow files, by path from the repository's root.
#[derive(Clone)]
pub struct Sources {
    /// `apps/<id>/workflows/`.
    pub dir: String,
    pub files: Arc<BTreeMap<String, String>>,
}

impl Sources {
    /// The path of workflow `name`: `apps/<id>/workflows/<name>.js`.
    pub fn path_of(&self, name: &str) -> String {
        format!("{}{name}.js", self.dir)
    }

    pub fn has(&self, name: &str) -> bool {
        self.files.contains_key(&self.path_of(name))
    }
}

/// `base`'s folder joined to `name`, with `.` and `..` resolved.
fn join(base: &str, name: &str) -> String {
    let mut parts: Vec<&str> = base.split('/').collect();
    parts.pop();
    for seg in name.split('/') {
        match seg {
            "." | "" => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    parts.join("/")
}

impl Resolver for Sources {
    fn resolve<'js>(
        &mut self,
        _ctx: &Ctx<'js>,
        base: &str,
        name: &str,
        _attributes: Option<ImportAttributes<'js>>,
    ) -> rquickjs::Result<String> {
        if base.starts_with("yen:") && (name == CTX || self.files.contains_key(name)) {
            return Ok(name.to_string());
        }
        let path = join(base, name);
        if (name.starts_with("./") || name.starts_with("../"))
            && path.starts_with(&self.dir)
            && self.files.contains_key(&path)
        {
            Ok(path)
        } else {
            Err(Error::new_resolving_message(
                base,
                name,
                "a workflow imports only the app's other workflow files",
            ))
        }
    }
}

impl Loader for Sources {
    fn load<'js>(
        &mut self,
        ctx: &Ctx<'js>,
        name: &str,
        _attributes: Option<ImportAttributes<'js>>,
    ) -> rquickjs::Result<Module<'js, Declared>> {
        let source = if name == CTX {
            CTX_SOURCE.to_string()
        } else {
            self.files.get(name).cloned().unwrap_or_default()
        };
        Module::declare(ctx.clone(), name, source)
    }
}

/// The line a QuickJS error's stack names in `path`.
pub fn line_in(stack: &str, path: &str) -> Option<usize> {
    let at = stack.find(&format!("{path}:"))? + path.len() + 1;
    let digits: String = stack[at..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

/// What went wrong, and on which line of `path`, as the engine tells it.
/// The engine stops a run past its budget with an `InternalError` whose
/// whole message is `interrupted`, answered as `None` in the second place.
pub fn said(caught: CaughtError, path: &str) -> (Option<usize>, Option<String>) {
    match caught {
        CaughtError::Exception(e) if e.message().as_deref() == Some("interrupted") => (None, None),
        CaughtError::Exception(e) => {
            let line = e.stack().and_then(|s| line_in(&s, path));
            let message = e.message().unwrap_or_else(|| "an error".to_string());
            (line, Some(message))
        }
        CaughtError::Value(v) => (None, Some(format!("threw {v:?}"))),
        CaughtError::Error(e) => (None, Some(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_relative_import_is_resolved_against_its_file() {
        assert_eq!(
            join("apps/n/workflows/a.js", "./b.js"),
            "apps/n/workflows/b.js"
        );
        assert_eq!(
            join("apps/n/workflows/x/a.js", "../b.js"),
            "apps/n/workflows/b.js"
        );
        assert_eq!(
            line_in(
                "    at remind (apps/n/workflows/a.js:7:3)",
                "apps/n/workflows/a.js"
            ),
            Some(7)
        );
        let s = Sources {
            dir: "apps/n/workflows/".into(),
            files: Arc::new(BTreeMap::from([(
                "apps/n/workflows/remind.js".to_string(),
                String::new(),
            )])),
        };
        assert_eq!(s.path_of("remind"), "apps/n/workflows/remind.js");
        assert!(s.has("remind"));
        assert!(!s.has("other"));
    }
}
