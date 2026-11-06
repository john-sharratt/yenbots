//! Every `/api/v1` route, defined once (`candle/docs/yenbot_app_host.md`
//! §6.4): the router is built from this table, `/api/v1/openapi.json` is
//! written from it, and a test holds `docs/api/` to it — a route without its
//! page, or a page naming a parameter or field its route does not take,
//! fails the build.

use std::sync::Arc;

use axum::routing::{delete, get, patch, post, put, MethodRouter};

use super::{admin, data, errors, events, me, openapi, reload};
use crate::app::App;

/// Where a parameter is read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum In {
    Path,
    Query,
}

/// The JSON type of a parameter or a body field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ty {
    Str,
    Int,
    Bool,
    List,
}

impl Ty {
    pub fn json(self) -> &'static str {
        match self {
            Ty::Str => "string",
            Ty::Int => "integer",
            Ty::Bool => "boolean",
            Ty::List => "array",
        }
    }
}

/// A parameter of the path or the query string.
#[derive(Debug, Clone, Copy)]
pub struct Param {
    pub name: &'static str,
    pub place: In,
    pub ty: Ty,
    pub required: bool,
    pub about: &'static str,
}

/// A field of the JSON body.
#[derive(Debug, Clone, Copy)]
pub struct Field {
    pub name: &'static str,
    pub ty: Ty,
    pub required: bool,
    pub about: &'static str,
}

/// One route: its method and path (`{name}` for a path parameter), the
/// `docs/api/<page>.md` it is described on, and what it takes.
pub struct Route {
    pub method: &'static str,
    pub path: &'static str,
    pub page: &'static str,
    pub summary: &'static str,
    pub params: &'static [Param],
    pub body: &'static [Field],
    pub handler: fn() -> MethodRouter<Arc<App>>,
}

impl Route {
    /// The path as the router spells it: `:name` for `{name}`.
    pub fn axum_path(&self) -> String {
        self.path.replace('{', ":").replace('}', "")
    }
}

const fn path(name: &'static str, about: &'static str) -> Param {
    Param {
        name,
        place: In::Path,
        ty: Ty::Str,
        required: true,
        about,
    }
}

const fn query(name: &'static str, ty: Ty, about: &'static str) -> Param {
    Param {
        name,
        place: In::Query,
        ty,
        required: false,
        about,
    }
}

const fn needs(name: &'static str, ty: Ty, about: &'static str) -> Field {
    Field {
        name,
        ty,
        required: true,
        about,
    }
}

const fn may(name: &'static str, ty: Ty, about: &'static str) -> Field {
    Field {
        name,
        ty,
        required: false,
        about,
    }
}

const APP: Param = path("app", "the app's id");
const COLL: Param = path("coll", "the collection, as the app's manifest names it");
const ID: Param = path("id", "the record's id");
const WHERE: Param = query(
    "where",
    Ty::Str,
    "clauses `field op value` joined by ` and `; ops `= != < <= > >= ~`",
);

/// Laid out as a table, one route to a block, so it reads as the reference
/// does.
#[rustfmt::skip]
pub const ROUTES: &[Route] = &[
    // The site.
    Route {
        method: "GET", path: "/api/v1/me", page: "site",
        summary: "Who the caller is, registering a signed-in person's account on first sight.",
        params: &[], body: &[], handler: || get(me::me),
    },
    Route {
        method: "GET", path: "/api/v1/apps", page: "site",
        summary: "The apps the caller may open, in the landing page's order.",
        params: &[], body: &[], handler: || get(data::apps),
    },
    Route {
        method: "GET", path: "/api/v1/apps/{app}", page: "site",
        summary: "One app as its pages are built from it: its collections, their fields, and what the caller may add to.",
        params: &[APP], body: &[], handler: || get(data::describe),
    },
    Route {
        method: "GET", path: "/api/v1/events/_reload", page: "site",
        summary: "Server-sent events: `reload` each time the site serves a new commit.",
        params: &[], body: &[], handler: || get(reload::reload),
    },
    Route {
        method: "GET", path: "/api/v1/events/{app}", page: "site",
        summary: "Server-sent events: a `change` for each write to the app's records the caller may see, `reset` to read everything again.",
        params: &[APP], body: &[], handler: || get(events::changes),
    },
    Route {
        method: "POST", path: "/api/v1/errors", page: "site",
        summary: "What went wrong in a browser, written to the app's log; repeats are counted.",
        params: &[],
        body: &[
            needs("app", Ty::Str, "the app's id, or `_site` for the platform's own pages"),
            needs("page", Ty::Str, "the page's address"),
            needs("message", Ty::Str, "what went wrong"),
            may("stack", Ty::Str, "where, as the browser tells it"),
            may("repeats", Ty::Int, "how many more times the page saw it since it last posted it"),
        ],
        handler: || post(errors::report),
    },
    Route {
        method: "GET", path: "/api/v1/openapi.json", page: "site",
        summary: "This API, described as OpenAPI 3.1.",
        params: &[], body: &[], handler: || get(openapi::openapi),
    },
    // An app's records.
    Route {
        method: "GET", path: "/api/v1/data/{app}/{coll}", page: "data",
        summary: "A page of the collection's records the caller may see.",
        params: &[
            APP, COLL, WHERE,
            query("order", Ty::Str, "a field, `-field` for descending; creation order when absent"),
            query("limit", Ty::Int, "records per page, 1 to 200; 50 when absent"),
            query("after", Ty::Str, "the cursor the last page answered"),
        ],
        body: &[], handler: || get(data::list),
    },
    Route {
        method: "POST", path: "/api/v1/data/{app}/{coll}", page: "data",
        summary: "Create a record, validated against the collection's fields.",
        params: &[APP, COLL], body: &[], handler: || post(data::create),
    },
    Route {
        method: "GET", path: "/api/v1/data/{app}/{coll}/_sum", page: "data",
        summary: "Counts, and the sum, average, least and greatest of one field, grouped.",
        params: &[
            APP, COLL, WHERE,
            query("group", Ty::Str, "one or two fields, comma-separated"),
            query("of", Ty::Str, "the number field `sum`, `avg`, `min` and `max` are taken over"),
        ],
        body: &[], handler: || get(data::sum),
    },
    Route {
        method: "GET", path: "/api/v1/data/{app}/{coll}/_search", page: "data",
        summary: "Full-text search over the collection's searchable fields, best first.",
        params: &[
            APP, COLL,
            Param { name: "q", place: In::Query, ty: Ty::Str, required: true, about: "the words; each matches as a prefix, accents ignored" },
            query("limit", Ty::Int, "how many, 1 to 200; 20 when absent"),
        ],
        body: &[], handler: || get(data::search),
    },
    Route {
        method: "GET", path: "/api/v1/data/{app}/{coll}/{id}", page: "data",
        summary: "One record.",
        params: &[APP, COLL, ID], body: &[], handler: || get(data::read),
    },
    Route {
        method: "PATCH", path: "/api/v1/data/{app}/{coll}/{id}", page: "data",
        summary: "Merge fields into a record; `null` removes one. `If-Match` names the version read.",
        params: &[APP, COLL, ID], body: &[], handler: || patch(data::patch),
    },
    Route {
        method: "DELETE", path: "/api/v1/data/{app}/{coll}/{id}", page: "data",
        summary: "Delete a record.",
        params: &[APP, COLL, ID], body: &[], handler: || delete(data::remove),
    },
    Route {
        method: "PUT", path: "/api/v1/data/{app}/{coll}/{id}/_share", page: "data",
        summary: "Who else may read or edit one private record; the owner's alone.",
        params: &[APP, COLL, ID],
        body: &[
            may("read", Ty::List, "`account:<id>` and `group:<name>` who may read it"),
            may("edit", Ty::List, "those who may also change it"),
        ],
        handler: || put(data::share),
    },
    Route {
        method: "POST", path: "/api/v1/data/{app}/_batch", page: "data",
        summary: "Creates, updates and deletes in one transaction, all or none.",
        params: &[APP],
        body: &[needs("writes", Ty::List, "each `{op: create|update|delete, coll, id?, data?, version?}`")],
        handler: || post(data::batch),
    },
    // Accounts and groups.
    Route {
        method: "GET", path: "/api/v1/admin/accounts", page: "admin-accounts",
        summary: "Every account, oldest first.",
        params: &[], body: &[], handler: || get(admin::accounts_list),
    },
    Route {
        method: "POST", path: "/api/v1/admin/accounts/{id}/{action}", page: "admin-accounts",
        summary: "Approve, refuse (a pending account), block or delete an account.",
        params: &[path("id", "the account's id"), path("action", "`approve`, `refuse`, `block` or `delete`")],
        body: &[], handler: || post(admin::accounts_act),
    },
    Route {
        method: "GET", path: "/api/v1/admin/groups", page: "admin-accounts",
        summary: "The groups and who is in each.",
        params: &[], body: &[], handler: || get(admin::groups_list),
    },
    Route {
        method: "POST", path: "/api/v1/admin/groups", page: "admin-accounts",
        summary: "Make a group.",
        params: &[], body: &[needs("name", Ty::Str, "lowercase letters, digits, `-` and `_`")],
        handler: || post(admin::groups_make),
    },
    Route {
        method: "DELETE", path: "/api/v1/admin/groups/{name}", page: "admin-accounts",
        summary: "Delete a group.",
        params: &[path("name", "the group")], body: &[], handler: || delete(admin::groups_drop),
    },
    Route {
        method: "POST", path: "/api/v1/admin/groups/{name}/members", page: "admin-accounts",
        summary: "Put an account in a group.",
        params: &[path("name", "the group")],
        body: &[needs("account", Ty::Str, "the account's id")],
        handler: || post(admin::members_add),
    },
    Route {
        method: "DELETE", path: "/api/v1/admin/groups/{name}/members/{account}", page: "admin-accounts",
        summary: "Take an account out of a group.",
        params: &[path("name", "the group"), path("account", "the account's id")],
        body: &[], handler: || delete(admin::members_remove),
    },
    Route {
        method: "PUT", path: "/api/v1/admin/registration", page: "admin-accounts",
        summary: "How a new account becomes active.",
        params: &[], body: &[needs("registration", Ty::Str, "`approve` or `open`")],
        handler: || put(admin::registration),
    },
    // Apps and their records.
    Route {
        method: "GET", path: "/api/v1/admin/apps", page: "admin-apps",
        summary: "Every app, with its visibility, shares, place and collections.",
        params: &[], body: &[], handler: || get(admin::apps_list),
    },
    Route {
        method: "PUT", path: "/api/v1/admin/apps/{id}/visibility", page: "admin-apps",
        summary: "Make an app public or private.",
        params: &[path("id", "the app's id")],
        body: &[needs("private", Ty::Bool, "whether only its shares may open it")],
        handler: || put(admin::apps_visibility),
    },
    Route {
        method: "PUT", path: "/api/v1/admin/apps/{id}/shares", page: "admin-apps",
        summary: "Who a private app is shared with, replacing the list.",
        params: &[path("id", "the app's id")],
        body: &[needs("who", Ty::List, "`account:<id>` and `group:<name>`")],
        handler: || put(admin::apps_shares),
    },
    Route {
        method: "PUT", path: "/api/v1/admin/apps/order", page: "admin-apps",
        summary: "The apps' order on the landing page.",
        params: &[], body: &[needs("order", Ty::List, "app ids, first first")],
        handler: || put(admin::apps_order),
    },
    Route {
        method: "GET", path: "/api/v1/admin/data/{app}/{coll}", page: "admin-apps",
        summary: "Any collection, every account's records, read-only.",
        params: &[
            APP, COLL, WHERE,
            query("order", Ty::Str, "a field, `-field` for descending"),
            query("limit", Ty::Int, "records per page, 1 to 200"),
            query("after", Ty::Str, "the cursor the last page answered"),
        ],
        body: &[], handler: || get(admin::data_all),
    },
    // Tokens.
    Route {
        method: "GET", path: "/api/v1/admin/tokens", page: "admin-tokens",
        summary: "The API tokens, by name, with their accounts and expiries; never a secret.",
        params: &[], body: &[], handler: || get(admin::tokens_list),
    },
    Route {
        method: "POST", path: "/api/v1/admin/tokens", page: "admin-tokens",
        summary: "Make a token for the caller's account; its secret is answered once.",
        params: &[],
        body: &[
            needs("name", Ty::Str, "what it is for, unique"),
            may("days", Ty::Int, "how long it lasts, 1 to 366; 365 when absent"),
        ],
        handler: || post(admin::tokens_make),
    },
    Route {
        method: "DELETE", path: "/api/v1/admin/tokens/{name}", page: "admin-tokens",
        summary: "Revoke a token.",
        params: &[path("name", "the token's name")], body: &[], handler: || delete(admin::tokens_revoke),
    },
    // Backups.
    Route {
        method: "GET", path: "/api/v1/admin/backups", page: "admin-backups",
        summary: "The nightly backups, oldest first, with their sizes.",
        params: &[], body: &[], handler: || get(admin::backups_list),
    },
    Route {
        method: "POST", path: "/api/v1/admin/backups/{date}/restore", page: "admin-backups",
        summary: "Restore one app's records from a backup — or, without `app`, the whole store.",
        params: &[path("date", "the backup's day, `YYYY-MM-DD`")],
        body: &[
            may("app", Ty::Str, "the app whose records come back; the whole store when absent"),
            may("collections", Ty::List, "only these of the app's collections"),
        ],
        handler: || post(admin::backups_restore),
    },
];
