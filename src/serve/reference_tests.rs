//! The reference held to the routes (§6.4): `docs/api/` describes every
//! route of [`ROUTES`] on its page, with every parameter and body field it
//! takes and no other, and a complete example; and the router serves every
//! route the table defines.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

use super::router;
use super::routes::ROUTES;
use super::tests::app;

/// One route's section of a page: the names its tables give — a row
/// ``| `name` | …`` — and whether it shows an example.
struct Section {
    names: BTreeSet<String>,
    example: bool,
}

fn reference() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("docs")
        .join("api")
}

/// The sections of every page in `docs/api/`, keyed `(page, "METHOD /path")`.
fn sections() -> BTreeMap<(String, String), Section> {
    let mut out = BTreeMap::new();
    for entry in std::fs::read_dir(reference()).unwrap() {
        let path = entry.unwrap().path();
        let page = path.file_stem().unwrap().to_string_lossy().to_string();
        if path.extension().is_none_or(|e| e != "md") || page == "index" {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        let mut at: Option<(String, String)> = None;
        for line in text.lines() {
            if let Some(h) = line.strip_prefix("### `").and_then(|h| h.strip_suffix('`')) {
                let key = (page.clone(), h.to_string());
                assert!(!out.contains_key(&key), "{page}.md describes `{h}` twice");
                let fresh = Section {
                    names: BTreeSet::new(),
                    example: false,
                };
                out.insert(key.clone(), fresh);
                at = Some(key);
                continue;
            }
            if line.starts_with("## ") {
                at = None;
            }
            let Some(key) = &at else { continue };
            let s = out.get_mut(key).unwrap();
            if let Some(rest) = line.strip_prefix("| `") {
                s.names.insert(rest.split('`').next().unwrap().to_string());
            }
            if line.starts_with("```") {
                s.example = true;
            }
        }
    }
    out
}

/// **Every route has its section, and every section is a route**, naming
/// exactly what the route takes, with an example.
#[test]
fn the_reference_describes_exactly_the_routes() {
    let mut pages = sections();
    for r in ROUTES {
        let route = format!("{} {}", r.method, r.path);
        let Some(s) = pages.remove(&(r.page.to_string(), route.clone())) else {
            panic!("docs/api/{}.md has no section for `{route}`", r.page);
        };
        let takes: BTreeSet<String> = r
            .params
            .iter()
            .map(|p| p.name)
            .chain(r.body.iter().map(|f| f.name))
            .map(String::from)
            .collect();
        assert_eq!(s.names, takes, "{route}: the page's names, the route's");
        assert!(s.example, "{route}: no example");
    }
    let left: Vec<_> = pages.keys().collect();
    assert!(left.is_empty(), "sections of no route: {left:?}");
}

/// **The index names every page.**
#[test]
fn the_index_names_every_page() {
    let index = std::fs::read_to_string(reference().join("index.md")).unwrap();
    let pages: BTreeSet<&str> = ROUTES.iter().map(|r| r.page).collect();
    for p in pages {
        let link = format!("({p}.md)");
        assert!(index.contains(&link), "the index does not link {p}.md");
    }
}

/// **The router serves every route the table defines**: each answers its
/// own JSON — never the router's bare 404 or 405.
#[tokio::test]
async fn the_router_serves_every_route() {
    let app = app();
    for r in ROUTES {
        let uri = r
            .path
            .split('/')
            .map(|seg| if seg.starts_with('{') { "x" } else { seg })
            .collect::<Vec<_>>()
            .join("/");
        let req = Request::builder()
            .method(r.method)
            .uri(&uri)
            .body(Body::empty())
            .unwrap();
        let res = router(app.clone()).oneshot(req).await.unwrap();
        let route = format!("{} {uri}", r.method);
        assert_ne!(res.status(), StatusCode::METHOD_NOT_ALLOWED, "{route}");
        if uri.ends_with("_reload") {
            assert_eq!(res.status(), StatusCode::OK);
            continue;
        }
        let body = to_bytes(res.into_body(), 1 << 20).await.unwrap();
        let v: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
        assert!(v.is_object(), "{route}: answered no JSON of its own");
    }
}
