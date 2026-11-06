//! The routes (`candle/docs/yenbot_app_host.md` §5.2, §6):
//!
//! | Route | |
//! |---|---|
//! | `GET /` | the landing page (`web/index.html`) |
//! | `GET /_/…` | the platform's web files, embedded ([`web`]) |
//! | `GET /api/v1/me` | who the caller is, registering their account ([`me`]) |
//! | `GET /api/v1/events/_reload` | an event each time the site serves a new commit ([`reload`]) |
//! | `GET /v1/status` | the site's health ([`status`]) |
//! | `GET /v1/logs`, `/v1/logs/sources` | the site as a log source |
//!
//! The gateway serves `web/` from the live checkout too, so a change there is
//! live without a release; these are the same files, built in.

pub mod me;
pub mod reload;
pub mod status;
pub mod web;

use std::sync::Arc;

use axum::routing::get;
use axum::Router;

use crate::app::App;
use crate::logs::api;

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/", get(web::landing))
        .route("/_/*path", get(web::platform_file))
        .route("/api/v1/me", get(me::me))
        .route("/api/v1/events/_reload", get(reload::reload))
        .route("/v1/status", get(status::status))
        .route("/v1/logs", get(api::logs))
        .route("/v1/logs/sources", get(api::sources))
        .with_state(app)
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::Duration;

    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use futures_util::StreamExt;
    use serde_json::Value;
    use tower::ServiceExt;

    use super::*;
    use crate::config::Config;
    use crate::logs::{Level, Line, Logs};
    use crate::publish::Reloads;
    use crate::store::Store;
    use crate::version::BuildInfo;

    fn app() -> Arc<App> {
        let store = Arc::new(Store::in_memory().unwrap());
        Arc::new(App {
            cfg: Config::from_yaml(
                "data_dir: d\nlive_dir: l\nadmins: [yen@example.com]\nbus: {url: u, key_file: k}\n",
                Path::new("."),
            )
            .unwrap(),
            logs: Logs::new(store.clone()),
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
        assert_eq!(body["store"]["schema"], 1);
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
