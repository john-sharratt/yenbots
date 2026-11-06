//! `GET /api/v1/openapi.json` (§6.4): the API as OpenAPI 3.1, written from
//! the route table, so it describes exactly the routes the router serves.

use std::future::{ready, Ready};
use std::sync::Arc;

use axum::extract::State;
use axum::response::Response;
use serde_json::{json, Map, Value};

use super::reply::ok;
use super::routes::{In, Route, ROUTES};
use crate::app::App;

/// The route serving the description. Its future is named rather than
/// `async fn`'s opaque one: the table holds this handler, and an opaque type
/// whose body reads the table would be defined in terms of itself.
pub fn openapi(State(s): State<Arc<App>>) -> Ready<Response> {
    ready(ok(describe(ROUTES, &s.build.label())))
}

/// One operation: its summary, its page as a tag, what it takes, and the
/// answers every route may give.
fn operation(r: &Route) -> Value {
    let params: Vec<Value> = r
        .params
        .iter()
        .map(|p| {
            json!({
                "name": p.name,
                "in": match p.place { In::Path => "path", In::Query => "query" },
                "required": p.required,
                "description": p.about,
                "schema": {"type": p.ty.json()},
            })
        })
        .collect();
    let mut op = json!({
        "operationId": format!("{} {}", r.method, r.path),
        "summary": r.summary,
        "tags": [r.page],
        "parameters": params,
        "responses": {
            "200": {"description": "the answer, as JSON"},
            "401": {"$ref": "#/components/responses/Error"},
            "403": {"$ref": "#/components/responses/Error"},
            "404": {"$ref": "#/components/responses/Error"},
        },
    });
    if !r.body.is_empty() {
        let mut props = Map::new();
        for f in r.body {
            let prop = json!({"type": f.ty.json(), "description": f.about});
            props.insert(f.name.into(), prop);
        }
        let required: Vec<&str> = r
            .body
            .iter()
            .filter(|f| f.required)
            .map(|f| f.name)
            .collect();
        op["requestBody"] = json!({
            "required": !required.is_empty(),
            "content": {"application/json": {"schema": {
                "type": "object", "properties": props, "required": required,
            }}},
        });
    } else if matches!(r.method, "POST" | "PATCH") {
        // A record's fields are the app's own, declared in its manifest.
        op["requestBody"] = json!({
            "content": {"application/json": {"schema": {"type": "object"}}},
        });
    }
    op
}

/// The description of `routes`, for the build `version`.
pub fn describe(routes: &[Route], version: &str) -> Value {
    let mut paths = Map::new();
    for r in routes {
        let entry = paths.entry(r.path).or_insert_with(|| json!({}));
        entry[r.method.to_lowercase()] = operation(r);
    }
    json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Yen Bots",
            "version": version,
            "description": "Every route acts as the account the request carries: a signed-in \
                            browser's, or an API token's (`Authorization: Bearer …`). A browser \
                            write also sends `X-Yen-App`. The reference is docs/api/.",
        },
        "servers": [{"url": "https://yenbot.com.au"}],
        "security": [{"token": []}, {}],
        "components": {
            "securitySchemes": {"token": {"type": "http", "scheme": "bearer"}},
            "responses": {"Error": {
                "description": "`{error, detail, field?}`",
                "content": {"application/json": {"schema": {
                    "type": "object",
                    "properties": {
                        "error": {"type": "string"},
                        "detail": {"type": "string"},
                        "field": {"type": "string"},
                    },
                    "required": ["error", "detail"],
                }}},
            }},
        },
        "paths": paths,
    })
}

#[cfg(test)]
mod tests {
    use axum::routing::get;

    use super::*;
    use crate::serve::routes::{Field, Param, Ty};

    async fn nothing() {}

    /// **A route is described exactly as the table defines it.**
    #[test]
    fn a_route_is_described_as_defined() {
        let routes = [Route {
            method: "POST",
            path: "/api/v1/admin/groups/{name}/members",
            page: "admin-accounts",
            summary: "Put an account in a group.",
            params: &[Param {
                name: "name",
                place: In::Path,
                ty: Ty::Str,
                required: true,
                about: "the group",
            }],
            body: &[Field {
                name: "account",
                ty: Ty::Str,
                required: true,
                about: "the account's id",
            }],
            handler: || get(nothing),
        }];
        let d = describe(&routes, "3-abc");
        assert_eq!(d["info"]["version"], "3-abc");
        assert_eq!(
            d["paths"]["/api/v1/admin/groups/{name}/members"]["post"],
            json!({
                "operationId": "POST /api/v1/admin/groups/{name}/members",
                "summary": "Put an account in a group.",
                "tags": ["admin-accounts"],
                "parameters": [{
                    "name": "name", "in": "path", "required": true,
                    "description": "the group", "schema": {"type": "string"},
                }],
                "requestBody": {
                    "required": true,
                    "content": {"application/json": {"schema": {
                        "type": "object",
                        "properties": {"account": {"type": "string", "description": "the account's id"}},
                        "required": ["account"],
                    }}},
                },
                "responses": {
                    "200": {"description": "the answer, as JSON"},
                    "401": {"$ref": "#/components/responses/Error"},
                    "403": {"$ref": "#/components/responses/Error"},
                    "404": {"$ref": "#/components/responses/Error"},
                },
            })
        );
    }

    /// **Every route of the table is in the description**, once, under its
    /// method.
    #[test]
    fn every_route_is_described() {
        let d = describe(ROUTES, "v");
        let mut n = 0;
        for (_, ops) in d["paths"].as_object().unwrap() {
            n += ops.as_object().unwrap().len();
        }
        assert_eq!(n, ROUTES.len());
        for r in ROUTES {
            let op = &d["paths"][r.path][r.method.to_lowercase()];
            assert!(op.is_object(), "{} {}", r.method, r.path);
        }
    }
}
