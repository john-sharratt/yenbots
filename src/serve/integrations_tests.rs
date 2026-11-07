//! An app's secrets and inbound hooks through the API (§19): keys set and
//! listed by name alone, a hook's token made once, and a call in proven by
//! its token or its signature — or refused and logged.

use std::path::Path;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use hmac::{Hmac, Mac};
use serde_json::{json, Value};
use sha2::Sha256;
use tower::ServiceExt;

use super::router;
use super::tests::{answer, call, serving, HOME};
use crate::app::App;
use crate::apps::Apps;
use crate::store::Store;

const YEN: (&str, &str) = ("1", "yen@example.com");

/// An admin's call, as the admin page makes it.
async fn as_yen(
    app: &Arc<App>,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    call(app, method, path, YEN, body, true).await
}

/// A call in to `path`, carrying `body` and the headers `with`.
async fn call_in(app: &Arc<App>, path: &str, with: &[(&str, &str)]) -> (StatusCode, Value) {
    let mut req = Request::post(path).header("content-type", "application/json");
    for (k, v) in with {
        req = req.header(*k, *v);
    }
    let res = router(app.clone())
        .oneshot(req.body(Body::from(BODY)).unwrap())
        .await
        .unwrap();
    answer(res).await
}

const BODY: &str = r#"{"name": "Mai"}"#;

fn signed(key: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(key.as_bytes()).unwrap();
    mac.update(BODY.as_bytes());
    let sig: String = mac
        .finalize()
        .into_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    format!("sha256={sig}")
}

fn site(dir: &Path) -> Arc<App> {
    let home = dir.join("apps").join("home");
    std::fs::create_dir_all(home.join("workflows").join("hooks")).unwrap();
    std::fs::write(home.join("app.yaml"), HOME).unwrap();
    std::fs::write(
        home.join("workflows").join("hooks").join("form.js"),
        "export default async function form(ctx, req) {\n\
           return { name: req.json.name, method: req.method, query: req.query,\n\
                    auth: req.headers.authorization ?? null, as: ctx.account };\n\
         }",
    )
    .unwrap();
    serving(Apps::open(dir, Arc::new(Store::in_memory().unwrap())))
}

/// **A key is set and listed by name, never answered; only an admin may.**
#[tokio::test]
async fn secrets_are_set_and_listed_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let app = site(dir.path());
    let at = "/api/v1/admin/secrets/home/api-key";
    let cousin = ("2", "c@example.com");
    let (status, _) = call(&app, "PUT", at, cousin, Some(json!({"value": "k"})), true).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, set) = as_yen(&app, "PUT", at, Some(json!({"value": "k-123"}))).await;
    assert_eq!(status, StatusCode::OK, "{set}");
    let (status, listed) = as_yen(&app, "GET", "/api/v1/admin/secrets/home", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(listed["secrets"][0]["name"], "api-key");
    assert!(!listed.to_string().contains("k-123"), "never answered");

    let x = Some(json!({"value": "x"}));
    let (status, bad) = as_yen(&app, "PUT", "/api/v1/admin/secrets/home/API", x.clone()).await;
    assert_eq!(
        (status, bad["error"].as_str()),
        (StatusCode::BAD_REQUEST, Some("bad_secret"))
    );
    let (status, _) = as_yen(&app, "PUT", "/api/v1/admin/secrets/nope/k", x).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(as_yen(&app, "DELETE", at, None).await.0, StatusCode::OK);
    assert_eq!(
        as_yen(&app, "DELETE", at, None).await.0,
        StatusCode::NOT_FOUND
    );
}

/// **A hook runs as the app for a call carrying its token or a signature of
/// its body; a call with neither is refused and written to the app's log;
/// a revoked token works no more.**
#[tokio::test]
async fn a_hook_runs_for_a_proven_call_only() {
    let dir = tempfile::tempdir().unwrap();
    let app = site(dir.path());
    let form = "/hooks/home/form";

    let (status, refused) = call_in(&app, form, &[]).await;
    assert_eq!(
        (status, refused["error"].as_str()),
        (StatusCode::UNAUTHORIZED, Some("unsigned"))
    );
    let logged: String = app
        .store
        .conn()
        .query_row(
            "SELECT message FROM log_lines WHERE target = 'hook'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        logged,
        "hook `form`: a POST call with neither its token nor a signature was refused"
    );
    assert_eq!(
        call_in(&app, "/hooks/home/nope", &[]).await.0,
        StatusCode::NOT_FOUND
    );

    let (_, listed) = as_yen(&app, "GET", "/api/v1/admin/hooks/home", None).await;
    assert_eq!(
        listed["hooks"],
        json!([{"hook": "form", "url": "/hooks/home/form", "token_made_ms": null,
                "signing_secret": "hook-form"}])
    );
    let (status, made) = as_yen(&app, "POST", "/api/v1/admin/hooks/home/form", None).await;
    assert_eq!(status, StatusCode::OK, "{made}");
    let token = made["token"].as_str().unwrap().to_string();
    let (status, ran) = call_in(&app, &format!("{form}?token={token}&x=1"), &[]).await;
    assert_eq!(status, StatusCode::OK, "{ran}");
    assert_eq!(
        ran,
        json!({"ok": true, "value": {"name": "Mai", "method": "POST", "query": {"x": "1"},
                                     "auth": null, "as": null}})
    );
    let bearer = format!("Bearer {token}");
    let (status, ran) = call_in(&app, form, &[("authorization", &bearer)]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(ran["value"]["auth"], Value::Null, "withheld");

    let jefe = Some(json!({"value": "Jefe"}));
    as_yen(&app, "PUT", "/api/v1/admin/secrets/home/hook-form", jefe).await;
    let sig = signed("Jefe");
    let (status, ran) = call_in(&app, form, &[("x-yen-signature", &sig)]).await;
    assert_eq!(
        (status, &ran["value"]["name"]),
        (StatusCode::OK, &json!("Mai"))
    );
    let hub = call_in(&app, form, &[("x-hub-signature-256", &sig)]).await;
    assert_eq!(hub.0, StatusCode::OK);
    let forged = signed("Jeff");
    let (status, _) = call_in(&app, form, &[("x-yen-signature", &forged)]).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _) = as_yen(&app, "DELETE", "/api/v1/admin/hooks/home/form", None).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = call_in(&app, &format!("{form}?token={token}"), &[]).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = as_yen(&app, "POST", "/api/v1/admin/hooks/home/nope", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
