//! The routes (`candle/docs/yenbot_app_host.md` §5.2, §6):
//!
//! | Route | |
//! |---|---|
//! | `GET /` | the landing page (`web/index.html`) |
//! | `GET /_/…` | the platform's web files, the framework and the design system, embedded ([`web`]) |
//! | `GET /_/theme/{id}.css` | an app's theme, from its manifest ([`theme`]) |
//! | `GET /_design` | the design system's catalogue |
//! | `GET /a/{id}/…` | an app's files and pages, from the live checkout ([`app_files`]) |
//! | `/api/v1/…` | the API, every route defined once in [`routes::ROUTES`] |
//! | `GET /v1/status` | the site's health ([`status`]) |
//! | `GET /v1/logs`, `/v1/logs/sources` | the site as a log source |
//!
//! A handler's refusal is the `Response` it answers with, so `Result<_,
//! Response>` is this layer's shape; boxing it would allocate on every
//! refusal to save a move.
#![allow(clippy::result_large_err)]

pub mod admin;
pub mod app_files;
pub mod asset;
pub mod data;
pub mod errors;
pub mod events;
pub mod me;
pub mod openapi;
pub mod reload;
pub mod reply;
pub mod routes;
pub mod status;
pub mod theme;
pub mod web;

#[cfg(test)]
mod reference_tests;

use std::sync::Arc;

use axum::routing::get;
use axum::Router;

use crate::app::App;
use crate::logs::api;
use routes::ROUTES;

pub fn router(app: Arc<App>) -> Router {
    let mut r = Router::new()
        .route("/", get(web::landing))
        .route("/_/*path", get(web::platform_file))
        .route("/_design", get(web::design))
        .route("/_design/*path", get(web::design_file))
        .route("/a/:id", get(app_files::bare))
        .route("/a/:id/", get(app_files::index))
        .route("/a/:id/*path", get(app_files::file))
        .route("/v1/status", get(status::status))
        .route("/v1/logs", get(api::logs))
        .route("/v1/logs/sources", get(api::sources));
    // Two routes on one path are one method router, merged.
    for route in ROUTES {
        r = r.route(&route.axum_path(), (route.handler)());
    }
    r.with_state(app)
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::Duration;

    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use axum::response::Response;
    use futures_util::StreamExt;
    use serde_json::{json, Value};
    use tower::ServiceExt;

    use super::*;
    use crate::apps::manifest::Manifest;
    use crate::apps::theme::Theme;
    use crate::apps::Apps;
    use crate::config::Config;
    use crate::data::changes::{Changes, Feed};
    use crate::logs::browser::Repeats;
    use crate::logs::{Level, Line, Logs};
    use crate::publish::Reloads;
    use crate::store::Store;
    use crate::version::BuildInfo;

    const HOME: &str =
        "id: home\nname: Home\nstatus: live\ncollections:\n  notes:\n    scope: private\n    \
                        fields:\n      text: { type: text, required: true }\n    search: [text]\n";

    fn serving(apps: Apps) -> Arc<App> {
        let store = Arc::new(Store::in_memory().unwrap());
        Arc::new(App {
            cfg: Config::from_yaml(
                "data_dir: d\nlive_dir: l\nadmins: [yen@example.com]\nbus: {url: u, key_file: k}\n",
                Path::new("."),
            )
            .unwrap(),
            logs: Logs::new(store.clone()),
            apps,
            reloads: Reloads::new(),
            changes: Changes::new(),
            repeats: Repeats::new(),
            store,
            build: BuildInfo::this(),
            started_ms: 1,
        })
    }

    pub(super) fn app() -> Arc<App> {
        serving(Apps::of(vec![Manifest::parse(HOME, "home").unwrap()]))
    }

    async fn get_as(app: &Arc<App>, path: &str, who: Option<(&str, &str)>) -> (StatusCode, Value) {
        let mut req = Request::get(path);
        if let Some((sub, email)) = who {
            req = req
                .header("x-tokera-user", sub)
                .header("x-tokera-provider", "google")
                .header("x-tokera-email", email)
                .header("x-tokera-name", "N");
        }
        let res = router(app.clone())
            .oneshot(req.body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = res.status();
        let body = to_bytes(res.into_body(), 1 << 20).await.unwrap();
        (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
    }

    /// **The site answers who you are, registering you on first sight**:
    /// signed out, a newcomer pending, an admin active.
    #[tokio::test]
    async fn me_registers_and_answers() {
        let app = app();
        let (_, anon) = get_as(&app, "/api/v1/me", None).await;
        assert_eq!(anon["signed_in"], false);
        assert_eq!(anon["site"], "Yen Bots");
        let (_, cousin) = get_as(&app, "/api/v1/me", Some(("2", "c@example.com"))).await;
        assert_eq!(cousin["account"]["status"], "pending");
        let (_, yen) = get_as(&app, "/api/v1/me", Some(("1", "yen@example.com"))).await;
        assert_eq!(
            (
                yen["account"]["status"].as_str(),
                yen["account"]["admin"].as_bool()
            ),
            (Some("active"), Some(true))
        );
    }

    /// **The logs are an admin's**, and read as a log source answers.
    #[tokio::test]
    async fn the_logs_are_an_admins() {
        let app = app();
        app.logs
            .write(&[Line {
                at_ms: 5,
                level: Level::Error,
                source: "yenbot/recipes".into(),
                target: "browser".into(),
                message: "TypeError".into(),
            }])
            .unwrap();
        assert_eq!(
            get_as(&app, "/v1/logs", None).await.0,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            get_as(&app, "/v1/logs", Some(("2", "c@example.com")))
                .await
                .0,
            StatusCode::FORBIDDEN
        );
        let (status, found) =
            get_as(&app, "/v1/logs?grep=Type", Some(("1", "yen@example.com"))).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(found["lines"].as_array().unwrap().len(), 1);
        let (_, sources) = get_as(&app, "/v1/logs/sources", Some(("1", "yen@example.com"))).await;
        assert_eq!(sources["sources"][1]["name"], "yenbot/recipes");
    }

    #[tokio::test]
    async fn the_status_says_ready_with_the_build() {
        let (status, body) = get_as(&app(), "/v1/status", None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["state"], "ready");
        assert_eq!(body["store"]["schema"], 2);
    }

    async fn call(
        app: &Arc<App>,
        method: &str,
        path: &str,
        who: (&str, &str),
        body: Option<Value>,
        framework: bool,
    ) -> (StatusCode, Value) {
        let mut req = Request::builder()
            .method(method)
            .uri(path)
            .header("x-tokera-user", who.0)
            .header("x-tokera-provider", "google")
            .header("x-tokera-email", who.1)
            .header("x-tokera-name", "N")
            .header("content-type", "application/json");
        if framework {
            req = req.header("x-yen-app", "home");
        }
        let body = body.map_or_else(Body::empty, |b| Body::from(b.to_string()));
        let res = router(app.clone()).oneshot(req.body(body).unwrap());
        answer(res.await.unwrap()).await
    }

    async fn answer(res: Response) -> (StatusCode, Value) {
        let status = res.status();
        let bytes = to_bytes(res.into_body(), 1 << 20).await.unwrap();
        let v = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (status, v)
    }

    const NOTES: &str = "/api/v1/data/home/notes";

    /// **A record is written through the framework, read back, and kept
    /// private to its owner**; a write without the framework's header is
    /// refused.
    #[tokio::test]
    async fn records_are_written_read_and_kept_private() {
        let app = app();
        let yen = ("1", "yen@example.com");
        let note = json!({"text": "Phở on Sunday"});
        let (status, _) = call(&app, "POST", NOTES, yen, Some(note.clone()), false).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "no X-Yen-App");
        let (status, made) = call(&app, "POST", NOTES, yen, Some(note), true).await;
        assert_eq!(status, StatusCode::CREATED);
        let id = made["id"].as_str().unwrap().to_string();
        let (_, page) = call(&app, "GET", NOTES, yen, None, false).await;
        assert_eq!(page["docs"][0]["data"]["text"], "Phở on Sunday");
        let search = format!("{NOTES}/_search?q=pho");
        let (_, found) = call(&app, "GET", &search, yen, None, false).await;
        assert_eq!(found["docs"].as_array().unwrap().len(), 1);
        let typo = Some(json!({"txt": "x"}));
        let (status, bad) = call(&app, "POST", NOTES, yen, typo, true).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(bad["field"], "txt");
        let (_, apps) = call(&app, "GET", "/api/v1/apps", yen, None, false).await;
        assert_eq!(apps["apps"][0]["id"], "home");
        // Another person cannot see it: here, one not yet approved.
        let cousin = ("2", "c@example.com");
        let one = format!("{NOTES}/{id}");
        let (status, _) = call(&app, "GET", &one, cousin, None, false).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "pending: no data");
    }

    async fn with_token(
        app: &Arc<App>,
        method: &str,
        path: &str,
        secret: &str,
    ) -> (StatusCode, Value) {
        let req = Request::builder()
            .method(method)
            .uri(path)
            .header("authorization", format!("Bearer {secret}"))
            .body(Body::empty())
            .unwrap();
        answer(router(app.clone()).oneshot(req).await.unwrap()).await
    }

    const ACCOUNTS: &str = "/api/v1/admin/accounts";

    /// **An admin runs the site through the API**: approves a newcomer,
    /// makes a token, and the token acts as her — admin routes included,
    /// with no framework header — until it is revoked. Nobody else reaches
    /// an admin route.
    #[tokio::test]
    async fn an_admin_runs_the_site_through_the_api() {
        let app = app();
        let (yen, mai) = (("1", "yen@example.com"), ("2", "mai@example.com"));
        get_as(&app, "/api/v1/me", Some(mai)).await;
        let (status, _) = call(&app, "GET", ACCOUNTS, mai, None, false).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "a pending account");
        let approve = format!("{ACCOUNTS}/2/approve");
        let (status, done) = call(&app, "POST", &approve, yen, None, true).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(done["done"], true);
        let (status, _) = call(&app, "GET", ACCOUNTS, mai, None, false).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "an active account, no admin");

        let ask = Some(json!({"name": "zen", "days": 30}));
        let (status, made) = call(&app, "POST", "/api/v1/admin/tokens", yen, ask, true).await;
        assert_eq!(status, StatusCode::OK);
        let secret = made["secret"].as_str().unwrap().to_string();
        let (_, listed) = call(&app, "GET", "/api/v1/admin/tokens", yen, None, false).await;
        assert_eq!(listed["tokens"][0]["name"], "zen");
        let shown = listed["tokens"][0].get("secret");
        assert!(shown.is_none(), "a secret is shown once");

        let (status, accounts) = with_token(&app, "GET", ACCOUNTS, &secret).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(accounts["accounts"][1]["status"], "active");
        let (_, me) = with_token(&app, "GET", "/api/v1/me", &secret).await;
        assert_eq!(me["account"]["id"], "1");
        assert_eq!(me["token"], "zen");
        let block = format!("{ACCOUNTS}/2/block");
        let (status, _) = with_token(&app, "POST", &block, &secret).await;
        assert_eq!(status, StatusCode::OK, "a token writes without X-Yen-App");

        let revoke = "/api/v1/admin/tokens/zen";
        let (status, _) = call(&app, "DELETE", revoke, yen, None, true).await;
        assert_eq!(status, StatusCode::OK);
        let (status, me) = with_token(&app, "GET", "/api/v1/me", &secret).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(me["error"], "bad_token");
        let (status, _) = with_token(&app, "GET", ACCOUNTS, &secret).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    /// **The OpenAPI description is served**, at the build serving it.
    #[tokio::test]
    async fn the_description_is_served() {
        let (status, d) = get_as(&app(), "/api/v1/openapi.json", None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(d["openapi"], "3.1.0");
        assert_eq!(d["info"]["version"], BuildInfo::this().label());
    }

    /// **An open page hears each move of the live checkout.**
    #[tokio::test]
    async fn an_open_page_hears_a_move() {
        let app = app();
        let res = router(app.clone())
            .oneshot(
                Request::get("/api/v1/events/_reload")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.headers()["content-type"], "text/event-stream");
        app.reloads.moved("4f1c");
        let mut body = res.into_body().into_data_stream();
        let chunk = tokio::time::timeout(Duration::from_secs(5), body.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(chunk, "event: reload\ndata: 4f1c\n\n");
    }

    /// The next event on an open feed, as its text.
    async fn next_event(res: Response) -> String {
        let mut body = res.into_body().into_data_stream();
        let chunk = tokio::time::timeout(Duration::from_secs(5), body.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        String::from_utf8(chunk.to_vec()).unwrap()
    }

    fn signed_in(path: &str, who: (&str, &str)) -> Request<Body> {
        Request::get(path)
            .header("x-tokera-user", who.0)
            .header("x-tokera-provider", "google")
            .header("x-tokera-email", who.1)
            .header("x-tokera-name", "N")
            .body(Body::empty())
            .unwrap()
    }

    /// **An open page hears of each write it may see, by id and version,
    /// never by its fields**, and of a restore as a reset.
    #[tokio::test]
    async fn an_open_page_hears_each_change() {
        let app = app();
        let yen = ("1", "yen@example.com");
        let feed = router(app.clone())
            .oneshot(signed_in("/api/v1/events/home", yen))
            .await
            .unwrap();
        assert_eq!(feed.headers()["content-type"], "text/event-stream");
        let note = Some(json!({"text": "Phở"}));
        let (_, made) = call(&app, "POST", NOTES, yen, note, true).await;
        let id = made["id"].as_str().unwrap();
        assert_eq!(
            next_event(feed).await,
            format!(
                "event: change\ndata: {{\"coll\":\"notes\",\"id\":\"{id}\",\"op\":\"created\",\"version\":1}}\n\n"
            )
        );
        let feed = router(app.clone())
            .oneshot(signed_in("/api/v1/events/home", yen))
            .await
            .unwrap();
        app.changes.send(Feed::Reset(Some("home".into())));
        assert_eq!(next_event(feed).await, "event: reset\ndata: {}\n\n");
        let (status, _) = get_as(&app, "/api/v1/events/nope", Some(yen)).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    /// **The framework reads what an app's pages are built from.**
    #[tokio::test]
    async fn an_app_is_described_to_its_pages() {
        let yen = ("1", "yen@example.com");
        let (status, d) = get_as(&app(), "/api/v1/apps/home", Some(yen)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            d["collections"]["notes"],
            json!({
                "scope": "private",
                "fields": {"text": {"type": "text", "required": true}},
                "search": ["text"],
                "may_add": true,
            })
        );
    }

    /// **A browser's error is written to its app's log, and a repeat is
    /// counted instead**; it comes from a signed-in page of the site.
    #[tokio::test]
    async fn a_browser_error_is_written_to_its_apps_log() {
        let app = app();
        let yen = ("1", "yen@example.com");
        let report = json!({
            "app": "home", "page": "https://yenbot.com.au/a/home/",
            "message": "TypeError: x is undefined", "stack": "at home.js:3",
        });
        let post = |framework| {
            call(
                &app,
                "POST",
                "/api/v1/errors",
                yen,
                Some(report.clone()),
                framework,
            )
        };
        assert_eq!(post(false).await.0, StatusCode::FORBIDDEN);
        assert_eq!(post(true).await.1, json!({"noted": "written"}));
        assert_eq!(post(true).await.1, json!({"noted": "counted"}));
        let (_, found) = get_as(&app, "/v1/logs?source=yenbot/home", Some(yen)).await;
        let lines = found["lines"].as_array().unwrap();
        assert_eq!(lines.len(), 1);
        let line = lines[0].as_str().unwrap();
        assert!(line.contains("ERROR yenbot/home browser: TypeError: x is undefined — at https://yenbot.com.au/a/home/ — account 1 on "));
        assert!(line.ends_with("\nat home.js:3"));
        let nowhere = json!({"app": "nope", "page": "p", "message": "m"});
        let (status, _) = call(&app, "POST", "/api/v1/errors", yen, Some(nowhere), true).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    async fn fetch(app: &Arc<App>, path: &str) -> (StatusCode, String) {
        let res = router(app.clone())
            .oneshot(Request::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = res.status();
        let location = res
            .headers()
            .get("location")
            .map(|l| l.to_str().unwrap().to_string());
        let body = to_bytes(res.into_body(), 1 << 20).await.unwrap();
        let text = String::from_utf8_lossy(&body).to_string();
        (status, location.unwrap_or(text))
    }

    /// **An app's files are served from its folder, every page of it is its
    /// `index.html` with its base, and the server's own files are not.**
    #[tokio::test]
    async fn an_apps_files_and_pages_are_served() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        std::fs::create_dir_all(home.join("workflows")).unwrap();
        std::fs::write(home.join("app.yaml"), HOME).unwrap();
        std::fs::write(home.join("index.html"), "<head></head><yen-app></yen-app>").unwrap();
        std::fs::write(home.join("app.js"), "export {};").unwrap();
        std::fs::write(home.join("workflows").join("remind.js"), "x").unwrap();
        let app = serving(Apps::open(dir.path()));
        let theme = theme::href("home", &Theme::default()).unwrap();
        let page = format!(
            "<head>\n  <base href=\"/a/home/\">\n  <link rel=\"stylesheet\" href=\"/_/yen.css\">\n  <link \
             rel=\"stylesheet\" href=\"{theme}\"></head><yen-app></yen-app>"
        );
        assert_eq!(
            fetch(&app, "/a/home").await,
            (StatusCode::PERMANENT_REDIRECT, "/a/home/".into())
        );
        assert_eq!(
            fetch(&app, "/a/home/").await,
            (StatusCode::OK, page.clone())
        );
        assert_eq!(fetch(&app, "/a/home/r/7").await, (StatusCode::OK, page));
        assert_eq!(
            fetch(&app, "/a/home/app.js").await,
            (StatusCode::OK, "export {};".into())
        );
        for hidden in [
            "/a/home/app.yaml",
            "/a/home/workflows/remind.js",
            "/a/home/nope.js",
            "/a/nope/",
        ] {
            assert_eq!(
                fetch(&app, hidden).await.0,
                StatusCode::NOT_FOUND,
                "{hidden}"
            );
        }
    }

    /// **An app's theme is served as its stylesheet**, an app with none as an
    /// empty one, and a page links it by its contents' digest, so an edit to
    /// the theme is a new address.
    #[tokio::test]
    async fn an_apps_theme_is_its_stylesheet() {
        let pink = format!("{HOME}theme:\n  accent: \"#b03a5b\"\n  radius: round\n");
        let app = serving(Apps::of(vec![
            Manifest::parse(&pink, "home").unwrap(),
            Manifest::parse(&HOME.replace("home", "plain"), "plain").unwrap(),
        ]));
        let (status, css) = fetch(&app, "/_/theme/home.css").await;
        assert_eq!(status, StatusCode::OK);
        assert!(css.contains("--yen-accent: #b03a5b;"), "{css}");
        assert!(
            css.contains("--yen-radius-button: var(--yen-radius-round-button);"),
            "{css}"
        );
        assert_eq!(
            fetch(&app, "/_/theme/plain.css").await,
            (
                StatusCode::OK,
                "/* This app's theme, from its manifest. */\n".into()
            )
        );
        for missing in ["/_/theme/nope.css", "/_/theme/home", "/_/theme/../yen.js"] {
            assert_eq!(
                fetch(&app, missing).await.0,
                StatusCode::NOT_FOUND,
                "{missing}"
            );
        }
        let a = theme::href("home", &app.apps.get("home").unwrap().theme).unwrap();
        let b = theme::href("home", &Theme::default()).unwrap();
        assert!(a.starts_with("/_/theme/home.css?v="), "{a}");
        assert_ne!(a, b);
    }

    /// **The catalogue and its files are served.**
    #[tokio::test]
    async fn the_catalogue_is_served() {
        let app = app();
        for path in [
            "/_design",
            "/_design/catalogue.js",
            "/_/yen.css",
            "/_/icons/sprite.svg",
        ] {
            assert_eq!(fetch(&app, path).await.0, StatusCode::OK, "{path}");
        }
        assert_eq!(fetch(&app, "/_design/nope").await.0, StatusCode::NOT_FOUND);
        assert_eq!(
            fetch(&app, "/_design/../index.html").await.0,
            StatusCode::NOT_FOUND
        );
    }

    #[tokio::test]
    async fn the_landing_page_and_its_files_are_served() {
        for path in ["/", "/_/site/landing.js", "/_/site/landing.css"] {
            let res = router(app())
                .oneshot(Request::get(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(res.status(), StatusCode::OK, "{path}");
        }
        let missing = router(app())
            .oneshot(Request::get("/_/nope.js").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    }
}
