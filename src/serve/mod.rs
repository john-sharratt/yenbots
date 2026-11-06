//! The routes (`candle/docs/yenbot_app_host.md` §5.2, §6):
//!
//! | Route | |
//! |---|---|
//! | `GET /` | the landing page (`web/index.html`) |
//! | `GET /_/…` | the platform's web files, embedded ([`web`]) |
//! | `/api/v1/…` | the API, every route defined once in [`routes::ROUTES`] |
//! | `GET /v1/status` | the site's health ([`status`]) |
//! | `GET /v1/logs`, `/v1/logs/sources` | the site as a log source |
//!
//! A handler's refusal is the `Response` it answers with, so `Result<_,
//! Response>` is this layer's shape; boxing it would allocate on every
//! refusal to save a move.
#![allow(clippy::result_large_err)]

pub mod admin;
pub mod data;
pub mod me;
pub mod openapi;
pub mod reload;
pub mod reply;
pub mod routes;
pub mod status;
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
    use crate::apps::Apps;
    use crate::config::Config;
    use crate::logs::{Level, Line, Logs};
    use crate::publish::Reloads;
    use crate::store::Store;
    use crate::version::BuildInfo;

    pub(super) fn app() -> Arc<App> {
        let store = Arc::new(Store::in_memory().unwrap());
        Arc::new(App {
            cfg: Config::from_yaml(
                "data_dir: d\nlive_dir: l\nadmins: [yen@example.com]\nbus: {url: u, key_file: k}\n",
                Path::new("."),
            )
            .unwrap(),
            logs: Logs::new(store.clone()),
            apps: Apps::of(vec![Manifest::parse(
                "id: home\nname: Home\nstatus: live\ncollections:\n  notes:\n    scope: private\n    \
                 fields:\n      text: { type: text, required: true }\n    search: [text]\n",
                "home",
            )
            .unwrap()]),
            reloads: Reloads::new(),
            store,
            build: BuildInfo::this(),
            started_ms: 1,
        })
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
