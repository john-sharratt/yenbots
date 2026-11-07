//! The routes (`candle/docs/yenbot_app_host.md` §5.2, §6):
//!
//! | Route | |
//! |---|---|
//! | `GET /` | the landing page (`web/index.html`) |
//! | `GET /_/…` | the platform's web files, the framework and the design system, embedded ([`web`]) |
//! | `GET /_/theme/{id}.css` | an app's theme, from its manifest ([`theme`]) |
//! | `GET /_/manifest.webmanifest`, `GET /sw.js` | the installable site: its manifest and service worker ([`install`]) |
//! | `GET /_design` | the design system's catalogue |
//! | `GET /_library` | the art library's page |
//! | `GET /_library/{kind}/{id}-{width}.webp` | a library picture at one of its sizes ([`library`]) |
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
pub mod ai;
pub mod app_files;
pub mod asset;
pub mod data;
pub mod errors;
pub mod events;
pub mod files;
pub mod install;
pub mod jobs;
pub mod library;
pub mod me;
pub mod notify;
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
        .route("/sw.js", get(install::service_worker))
        .route("/_/*path", get(web::platform_file))
        .route("/_design", get(web::design))
        .route("/_design/*path", get(web::design_file))
        .route("/_library", get(web::library))
        .route("/_library/:kind/:file", get(library::picture))
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
    use std::io::Cursor;
    use std::path::Path;
    use std::time::Duration;

    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use axum::response::Response;
    use futures_util::StreamExt;
    use image::{ImageFormat, Rgba, RgbaImage};
    use serde_json::{json, Value};
    use tower::ServiceExt;

    use base64::Engine;

    use super::*;
    use crate::ai::fake::FakeZen;
    use crate::ai::NOT_AI;
    use crate::apps::manifest::Manifest;
    use crate::apps::theme::Theme;
    use crate::apps::Apps;
    use crate::config::Config;
    use crate::data::changes::Feed;
    use crate::jobs::scheduler::sync_all;
    use crate::library::Library;
    use crate::logs::{Level, Line};
    use crate::store::backups::back_up;
    use crate::store::Store;
    use crate::version::BuildInfo;

    const HOME: &str =
        "id: home\nname: Home\nstatus: live\ncollections:\n  notes:\n    scope: private\n    \
                        fields:\n      text: { type: text, required: true }\n    search: [text]\n";

    fn serving(apps: Apps) -> Arc<App> {
        serving_with(apps, Library::new("l", "l/cache"))
    }

    fn serving_with(apps: Apps, library: Library) -> Arc<App> {
        let cfg = Config::from_yaml(
            "data_dir: d\nlive_dir: l\nadmins: [yen@example.com]\ntime_zone: Australia/Sydney\n\
             bus: {url: u, key_file: k}\n",
            Path::new("."),
        )
        .unwrap();
        let mut app = App::for_tests(cfg, Arc::new(Store::in_memory().unwrap()), apps);
        app.library = library;
        Arc::new(app)
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
        assert_eq!(anon["site"], "Yen Bot");
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
        assert_eq!(body["store"]["schema"], 5);
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
        let home = dir.path().join("apps").join("home");
        std::fs::create_dir_all(home.join("workflows")).unwrap();
        std::fs::write(home.join("app.yaml"), HOME).unwrap();
        std::fs::write(home.join("index.html"), "<head></head><yen-app></yen-app>").unwrap();
        std::fs::write(home.join("app.js"), "export {};").unwrap();
        std::fs::write(
            home.join("workflows").join("remind.js"),
            "export default async function remind() {}",
        )
        .unwrap();
        let app = serving(Apps::open(
            dir.path(),
            Arc::new(Store::in_memory().unwrap()),
        ));
        let theme = theme::href("home", &Theme::default()).unwrap();
        let installable: String = install::HEAD.iter().map(|l| format!("\n  {l}")).collect();
        let page = format!(
            "<head>\n  <base href=\"/a/home/\">\n  <link rel=\"stylesheet\" href=\"/_/yen.css\">\n  <link \
             rel=\"stylesheet\" href=\"{theme}\">{installable}</head><yen-app></yen-app>"
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

    /// **A workflow runs now through the API, as the caller or — for an
    /// admin — as the app; its job is listed, run now, held and released.**
    #[tokio::test]
    async fn workflows_and_jobs_through_the_api() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("apps").join("home");
        std::fs::create_dir_all(home.join("workflows")).unwrap();
        let schedules = "schedules:\n  - every: daily 07:30\n    run: hello\n  \
                         - when: {collection: notes, does: create}\n    run: noted\n";
        std::fs::write(
            home.join("workflows").join("noted.js"),
            "export default async function noted(ctx, { after }) { return after.text; }",
        )
        .unwrap();
        std::fs::write(home.join("app.yaml"), format!("{HOME}{schedules}")).unwrap();
        std::fs::write(
            home.join("workflows").join("hello.js"),
            "export default async function hello(ctx, { name }) {\n\
               ctx.log('hello', name);\n\
               if (name === 'x') throw new Error('not x');\n\
               return { to: name, as: ctx.account };\n\
             }",
        )
        .unwrap();
        let app = serving(Apps::open(
            dir.path(),
            Arc::new(Store::in_memory().unwrap()),
        ));
        let yen = ("1", "yen@example.com");
        let hello = "/api/v1/workflows/home/hello";
        let (status, ran) =
            call(&app, "POST", hello, yen, Some(json!({"name": "Mai"})), true).await;
        assert_eq!(status, StatusCode::OK, "{ran}");
        assert_eq!(ran["value"], json!({"to": "Mai", "as": "1"}));
        assert_eq!(ran["log"], json!(["hello Mai"]));
        let (status, failed) =
            call(&app, "POST", hello, yen, Some(json!({"name": "x"})), true).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(failed["detail"], "apps/home/workflows/hello.js:3 not x");
        let (status, _) = call(&app, "POST", "/api/v1/workflows/home/nope", yen, None, true).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        // A workflow the platform hands a record is not a person's to run.
        for handed in ["noted", "migrate-notes-2"] {
            let at = format!("/api/v1/workflows/home/{handed}");
            let (status, refused) = call(&app, "POST", &at, yen, None, true).await;
            assert_eq!(
                (status, refused["error"].as_str()),
                (StatusCode::FORBIDDEN, Some("not_yours_to_run")),
                "{handed}"
            );
        }
        let as_app = "/api/v1/admin/workflows/home/hello";
        let (_, ran) = call(&app, "POST", as_app, yen, Some(json!({"name": "Bo"})), true).await;
        assert_eq!(ran["value"], json!({"to": "Bo", "as": null}));

        sync_all(&app, 0).unwrap();
        let (_, mine) = call(&app, "GET", "/api/v1/jobs/home", yen, None, false).await;
        let jobs = mine["jobs"].as_array().unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(
            (jobs[0]["workflow"].as_str(), jobs[0]["state"].as_str()),
            (Some("hello"), Some("waiting"))
        );
        assert!(
            jobs[0].get("args").is_none(),
            "not shown to the app's pages"
        );
        let id = jobs[0]["id"].as_str().unwrap().to_string();
        let (_, all) = call(&app, "GET", "/api/v1/admin/jobs?app=home", yen, None, false).await;
        assert_eq!(all["jobs"][0]["args"], json!({}));
        let (_, bad) = call(
            &app,
            "GET",
            "/api/v1/admin/jobs?state=asleep",
            yen,
            None,
            false,
        )
        .await;
        assert_eq!(bad["error"], "bad_state");
        let act = |what: &str| format!("/api/v1/admin/jobs/{id}/{what}");
        let (_, held) = call(&app, "POST", &act("hold"), yen, None, true).await;
        assert_eq!(held["job"]["state"], "held");
        let (status, again) = call(&app, "POST", &act("hold"), yen, None, true).await;
        assert_eq!(
            (status, again["error"].as_str()),
            (StatusCode::CONFLICT, Some("refused"))
        );
        let (_, released) = call(&app, "POST", &act("release"), yen, None, true).await;
        assert_eq!(released["job"]["state"], "waiting");
        let (_, now) = call(&app, "POST", &act("run"), yen, None, true).await;
        assert!(now["job"]["next_ms"].as_i64().unwrap() > 1_000, "due now");
    }

    /// A site in `dir` serving `HOME` with `ai: true`, its Zen `zen`.
    fn with_ai(dir: &Path, zen: FakeZen) -> Arc<App> {
        let cfg = Config::from_yaml(
            "data_dir: data\nlive_dir: live\nadmins: [yen@example.com]\ntime_zone: UTC\nbus: {url: u, key_file: k}\n",
            dir,
        )
        .unwrap();
        let m = Manifest::parse(&format!("{HOME}ai: true\n"), "home").unwrap();
        let mut app = App::for_tests(
            cfg,
            Arc::new(Store::in_memory().unwrap()),
            Apps::of(vec![m]),
        );
        app.zen = Arc::new(zen);
        Arc::new(app)
    }

    /// **A page chats with the model and draws a picture, kept as one of
    /// its app's files; an app without `ai: true` is refused.**
    #[tokio::test]
    async fn an_app_chats_and_draws() {
        let dir = tempfile::tempdir().unwrap();
        let mut png = Cursor::new(Vec::new());
        RgbaImage::from_pixel(64, 32, Rgba([212, 87, 61, 255]))
            .write_to(&mut png, ImageFormat::Png)
            .unwrap();
        let png = png.into_inner();
        let zen = FakeZen::chats(vec![vec![format!(
            "data: {}",
            json!({"choices": [{"delta": {"content": "Phở."}}]})
        )]]);
        let app = with_ai(dir.path(), zen);
        let yen = ("1", "yen@example.com");
        let asked = json!({"messages": [{"role": "user", "content": "Dinner?"}]});
        let (status, said) = call(
            &app,
            "POST",
            "/api/v1/ai/chat",
            yen,
            Some(asked.clone()),
            true,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{said}");
        assert_eq!(said["content"], "Phở.");
        let kept: i64 = app
            .store
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM ai_calls WHERE kind = 'chat' AND account = '1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(kept, 1);

        let drawing = FakeZen::images(vec![vec![format!(
            "{{\"event\":\"done\",\"width\":64,\"height\":32,\"png_base64\":\"{}\",\"seed\":9}}",
            base64::engine::general_purpose::STANDARD.encode(&png)
        )]]);
        let app = with_ai(dir.path(), drawing);
        let draw = json!({"prompt": "A bowl", "width": 128, "height": 64});
        let (status, drawn) = call(&app, "POST", "/api/v1/ai/image", yen, Some(draw), true).await;
        assert_eq!(status, StatusCode::OK, "{drawn}");
        assert_eq!(drawn["seed"], 9);
        let thumb = drawn["file"]["thumb"].as_str().unwrap().to_string();
        let res = router(app.clone())
            .oneshot(signed_in(&thumb, yen))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.headers()["content-type"], "image/webp");

        let plain = serving(Apps::of(vec![Manifest::parse(HOME, "home").unwrap()]));
        let (status, no) = call(&plain, "POST", "/api/v1/ai/chat", yen, Some(asked), true).await;
        assert_eq!(
            (status, no["detail"].as_str()),
            (StatusCode::UNPROCESSABLE_ENTITY, Some(NOT_AI))
        );
    }

    /// **A file is kept for an app and served to those who may open it;
    /// one that is no picture or PDF is refused.**
    #[tokio::test]
    async fn an_apps_files_are_kept_and_served() {
        let dir = tempfile::tempdir().unwrap();
        let app = with_ai(dir.path(), FakeZen::default());
        let yen = ("1", "yen@example.com");
        get_as(&app, "/api/v1/me", Some(yen)).await;
        let post = |bytes: &'static [u8]| {
            Request::post("/api/v1/files/home?name=menu.pdf")
                .header("x-tokera-user", "1")
                .header("x-tokera-provider", "google")
                .header("x-tokera-email", "yen@example.com")
                .header("x-tokera-name", "N")
                .header("x-yen-app", "home")
                .body(Body::from(bytes))
                .unwrap()
        };
        let (status, kept) = answer(
            router(app.clone())
                .oneshot(post(b"%PDF-1.7 menu"))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{kept}");
        assert_eq!(
            (kept["name"].as_str(), kept["thumb"].clone()),
            (Some("menu.pdf"), Value::Null)
        );
        let url = kept["url"].as_str().unwrap().to_string();
        let res = router(app.clone())
            .oneshot(signed_in(&url, yen))
            .await
            .unwrap();
        assert_eq!(res.headers()["content-type"], "application/pdf");
        let bytes = to_bytes(res.into_body(), 1 << 20).await.unwrap();
        assert_eq!(&bytes[..], b"%PDF-1.7 menu");
        let (status, refused) =
            answer(router(app.clone()).oneshot(post(b"<svg/>")).await.unwrap()).await;
        assert_eq!(
            (status, refused["error"].as_str()),
            (StatusCode::UNPROCESSABLE_ENTITY, Some("bad_file"))
        );
        get_as(&app, "/api/v1/me", Some(("2", "c@example.com"))).await;
        let res = router(app.clone())
            .oneshot(signed_in(&url, ("2", "c@example.com")))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN, "not approved");
    }

    /// **A whole store restored brings its jobs back due**: the backup was
    /// written while its own job ran, and that job runs again.
    #[tokio::test]
    async fn a_restored_store_has_its_jobs_due() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = Config::from_yaml(
            "data_dir: data\nlive_dir: live\nadmins: [yen@example.com]\ntime_zone: UTC\nbus: {url: u, key_file: k}\n",
            dir.path(),
        )
        .unwrap();
        let store = Arc::new(Store::in_memory().unwrap());
        let app = Arc::new(App::for_tests(cfg, store.clone(), Apps::of(Vec::new())));
        let yen = ("1", "yen@example.com");
        get_as(&app, "/api/v1/me", Some(yen)).await;
        sync_all(&app, 0).unwrap();
        store
            .conn()
            .execute(
                "UPDATE jobs SET state = 'running' WHERE workflow = 'backup'",
                [],
            )
            .unwrap();
        let backups = app.cfg.backups_dir();
        let written = back_up(&store, &backups, 0, &app.cfg.zone())
            .unwrap()
            .unwrap();
        let day = written.file_stem().unwrap().to_string_lossy()["yenbot-".len()..].to_string();
        let at = format!("/api/v1/admin/backups/{day}/restore");
        let (status, body) = call(&app, "POST", &at, yen, None, true).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let state: String = store
            .conn()
            .query_row(
                "SELECT state FROM jobs WHERE workflow = 'backup'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(state, "waiting");
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

    /// **An admin reads an app's last check**; an app that is not there is
    /// said so, and no one else may ask.
    #[tokio::test]
    async fn an_admin_reads_an_apps_last_check() {
        let app = app();
        let yen = Some(("1", "yen@example.com"));
        let (status, body) = get_as(&app, "/api/v1/admin/apps/home/check", yen).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            body,
            json!({"app": "home", "ok": true, "commit": null, "serving": null, "refusals": [],
                   "migrating": [], "migrations": []})
        );
        let (status, body) = get_as(&app, "/api/v1/admin/apps/nope/check", yen).await;
        assert_eq!(
            (status, body["error"].clone()),
            (StatusCode::NOT_FOUND, json!("no_app"))
        );
        get_as(&app, "/api/v1/me", Some(("2", "c@example.com"))).await;
        let (status, _) = get_as(
            &app,
            "/api/v1/admin/apps/home/check",
            Some(("2", "c@example.com")),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
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

    /// **The library is served to anyone**: its page, each picture at a size
    /// it has, and the search, which refuses a kind or colour there is not.
    #[tokio::test]
    async fn the_library_is_served() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("live");
        std::fs::create_dir_all(root.join("library/tiles")).unwrap();
        std::fs::write(
            root.join("library/catalogue.yaml"),
            "tiles/bell-coral:\n  tags: [bell]\n  seed: 7\n  prompt: A bell.\n",
        )
        .unwrap();
        let mut png = Cursor::new(Vec::new());
        RgbaImage::from_pixel(64, 64, Rgba([212, 87, 61, 255]))
            .write_to(&mut png, ImageFormat::Png)
            .unwrap();
        std::fs::write(root.join("library/tiles/bell-coral.png"), png.into_inner()).unwrap();
        let library = Library::new(root, dir.path().join("cache"));
        library.reload();
        let app = serving_with(Apps::of(Vec::new()), library);

        assert_eq!(fetch(&app, "/_library").await.0, StatusCode::OK);
        assert_eq!(fetch(&app, "/_/site/library.js").await.0, StatusCode::OK);
        let res = router(app.clone())
            .oneshot(
                Request::get("/_library/tiles/bell-coral-128.webp")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.headers()["content-type"], "image/webp");
        for missing in [
            "/_library/tiles/bell-coral-100.webp",
            "/_library/tiles/cup-128.webp",
            "/_library/stickers/bell-coral-128.webp",
        ] {
            assert_eq!(
                fetch(&app, missing).await.0,
                StatusCode::NOT_FOUND,
                "{missing}"
            );
        }

        let (status, body) = get_as(&app, "/api/v1/library?kind=tiles&colour=coral", None).await;
        assert_eq!(status, StatusCode::OK);
        let pictures = body["pictures"].as_array().unwrap();
        assert_eq!(pictures.len(), 1);
        assert_eq!(pictures[0]["id"], "tiles/bell-coral");
        assert_eq!(pictures[0]["kind"], "tiles");
        assert_eq!(pictures[0]["colour"], "#d4573d");
        assert_eq!(pictures[0]["palette"], "coral");
        assert_eq!(pictures[0]["sizes"].as_array().unwrap().len(), 3);
        let (_, none) = get_as(&app, "/api/v1/library?tag=cup", None).await;
        assert_eq!(none, json!({"pictures": []}));
        let (status, bad) = get_as(&app, "/api/v1/library?kind=hats", None).await;
        assert_eq!(
            (status, bad["error"].clone()),
            (StatusCode::BAD_REQUEST, json!("bad_kind"))
        );
        let (status, bad) = get_as(&app, "/api/v1/library?colour=red", None).await;
        assert_eq!(
            (status, bad["error"].clone()),
            (StatusCode::BAD_REQUEST, json!("bad_colour"))
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
