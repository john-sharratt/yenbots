//! The apps over time through the API (§20): settings, opens and usage,
//! export and import, feedback, a newcomer told to Yen, and archiving and
//! restoring an app.

use std::path::Path;
use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

use super::router;
use super::tests::{call, HOME};
use crate::app::App;
use crate::apps::Apps;
use crate::config::Config;
use crate::store::Store;

const YEN: (&str, &str) = ("1", "yen@example.com");

/// Yen's call, as the site's own pages make it.
async fn as_yen(
    app: &Arc<App>,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    call(app, method, path, YEN, body, method != "GET").await
}

/// A call with a body that is not JSON, answered as text.
async fn raw(app: &Arc<App>, method: &str, path: &str, body: &str) -> (StatusCode, String) {
    let req = Request::builder()
        .method(method)
        .uri(path)
        .header("x-tokera-user", YEN.0)
        .header("x-tokera-provider", "google")
        .header("x-tokera-email", YEN.1)
        .header("x-tokera-name", "Yen")
        .header("x-yen-app", "home")
        .body(Body::from(body.to_string()))
        .unwrap();
    let res = router(app.clone()).oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = to_bytes(res.into_body(), 1 << 20).await.unwrap();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

/// A site whose apps and requests share one store, as the binary's do.
fn site(dir: &Path) -> Arc<App> {
    let home = dir.join("apps").join("home");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(home.join("app.yaml"), HOME).unwrap();
    let cfg = Config::from_yaml(
        "data_dir: data\nlive_dir: .\nadmins: [yen@example.com]\ntime_zone: UTC\n\
         bus: {url: u, key_file: k}\n",
        dir,
    )
    .unwrap();
    let store = Arc::new(Store::in_memory().unwrap());
    let apps = Apps::open(dir, store.clone());
    Arc::new(App::for_tests(cfg, store, apps))
}

/// **An account's settings are its own; each open is counted and read back
/// as usage.**
#[tokio::test]
async fn settings_opens_and_usage() {
    let dir = tempfile::tempdir().unwrap();
    let app = site(dir.path());
    let at = "/api/v1/settings/home";
    assert_eq!(as_yen(&app, "GET", at, None).await.1["settings"], json!({}));
    let units = json!({"units": "metric"});
    assert_eq!(
        as_yen(&app, "PUT", at, Some(units.clone())).await.0,
        StatusCode::OK
    );
    assert_eq!(as_yen(&app, "GET", at, None).await.1["settings"], units);
    let (status, bad) = as_yen(&app, "PUT", at, Some(json!([1]))).await;
    assert_eq!(
        (status, bad["error"].as_str()),
        (StatusCode::BAD_REQUEST, Some("bad_settings"))
    );

    for _ in 0..2 {
        as_yen(&app, "POST", "/api/v1/opens/home", None).await;
    }
    let (status, used) = as_yen(&app, "GET", "/api/v1/admin/usage?app=home", None).await;
    assert_eq!(status, StatusCode::OK, "{used}");
    let day = &used["usage"][0];
    assert_eq!(
        (day["opens"].as_i64(), day["accounts"].as_i64()),
        (Some(2), Some(1))
    );
    let (status, _) = as_yen(&app, "GET", "/api/v1/admin/usage?by=week", None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

/// **A spreadsheet's rows come in as records, and go out as JSON and as CSV
/// with their accents; an import refused keeps nothing.**
#[tokio::test]
async fn records_go_out_and_come_in() {
    let dir = tempfile::tempdir().unwrap();
    let app = site(dir.path());
    let csv_in = "/api/v1/import/home?format=csv&coll=notes";
    let (status, made) = raw(&app, "POST", csv_in, "text\nPhở\nChè\n").await;
    assert_eq!(status, StatusCode::OK, "{made}");
    assert_eq!(made, r#"{"app":"home","created":{"notes":2}}"#);
    let (status, refused) = raw(&app, "POST", csv_in, "text\nA\n\"B\n").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
    assert!(refused.contains("never closed"), "{refused}");

    let (_, out) = as_yen(&app, "GET", "/api/v1/export/home", None).await;
    let texts: Vec<&str> = out["collections"]["notes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["text"].as_str().unwrap())
        .collect();
    assert_eq!(texts, ["Phở", "Chè"], "nothing of the refused import");
    let (status, csv) = raw(&app, "GET", "/api/v1/export/home?format=csv&coll=notes", "").await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        csv.starts_with("\u{feff}id,owner,text,created_ms,updated_ms\r\n"),
        "{csv}"
    );
    assert!(csv.contains(",1,Phở,"), "{csv}");
    let (status, _) = raw(&app, "GET", "/api/v1/export/home?format=csv", "").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let back = json!({"collections": {"notes": out["collections"]["notes"].clone()}});
    let (status, copied) = as_yen(&app, "POST", "/api/v1/import/home", Some(back)).await;
    assert_eq!(
        (status, &copied["created"]["notes"]),
        (StatusCode::OK, &json!(2))
    );
}

/// **A newcomer's first visit tells Yen, once.**
#[tokio::test]
async fn yen_is_told_of_a_newcomer() {
    let dir = tempfile::tempdir().unwrap();
    let app = site(dir.path());
    as_yen(&app, "GET", "/api/v1/me", None).await;
    let mai = ("2", "mai@example.com");
    for _ in 0..2 {
        call(&app, "GET", "/api/v1/me", mai, None, false).await;
    }
    let (_, inbox) = as_yen(&app, "GET", "/api/v1/inbox", None).await;
    let items = inbox["items"].as_array().unwrap();
    assert_eq!(items.len(), 1, "{inbox}");
    assert_eq!(
        (items[0]["title"].as_str(), items[0]["open"].as_str()),
        (Some("N would like to join"), Some("/_admin#accounts"))
    );
}

/// **An app archived is served to no one until restored; a note sent from
/// it is listed for the admins.**
#[tokio::test]
async fn an_app_is_archived_and_restored() {
    let dir = tempfile::tempdir().unwrap();
    let app = site(dir.path());
    let note = Some(json!({"text": "More soups"}));
    let (status, _) = as_yen(&app, "POST", "/api/v1/feedback/home", note).await;
    assert_eq!(status, StatusCode::OK);
    let (_, notes) = as_yen(&app, "GET", "/api/v1/admin/feedback", None).await;
    assert_eq!(notes["feedback"][0]["text"], "More soups");

    let (status, _) = as_yen(&app, "POST", "/api/v1/admin/apps/home/archive", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(app.apps.get("home").is_none());
    let (_, listed) = as_yen(&app, "GET", "/api/v1/admin/archive", None).await;
    let first = &listed["archived"][0];
    assert_eq!(
        (first["app"].as_str(), first["reason"].as_str()),
        (Some("home"), Some("admin"))
    );
    let (status, _) = as_yen(&app, "GET", "/api/v1/settings/home", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "not served");

    let restore = "/api/v1/admin/apps/home/restore";
    assert_eq!(as_yen(&app, "POST", restore, None).await.0, StatusCode::OK);
    assert!(app.apps.get("home").is_some());
    let (status, again) = as_yen(&app, "POST", restore, None).await;
    assert_eq!(
        (status, again["error"].as_str()),
        (StatusCode::NOT_FOUND, Some("not_archived"))
    );
    let (status, gone) = as_yen(&app, "POST", "/api/v1/admin/apps/nope/restore", None).await;
    assert_eq!(
        (status, gone["error"].as_str()),
        (StatusCode::CONFLICT, Some("no_folder"))
    );
}
