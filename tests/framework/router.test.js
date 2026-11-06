import { appIdOf, match, routesOf, within } from "/_/framework/router.js";
import { eq, test } from "./harness.js";

const routes = routesOf({ "": "recipes-home", new: "recipe-new", "r/:id": "recipe-page", "r/:id/edit": "recipe-edit" });

test("an address names its page and parameters", () => {
  eq(match(routes, ""), { pattern: "", page: "recipes-home", params: {} });
  eq(match(routes, "new"), { pattern: "new", page: "recipe-new", params: {} });
  eq(match(routes, "r/7"), { pattern: "r/:id", page: "recipe-page", params: { id: "7" } });
  eq(match(routes, "r/7/edit/").params, { id: "7" }, "a trailing slash is the same page");
  eq(match(routes, "r/Ph%E1%BB%9F").params, { id: "Phở" });
  eq(match(routes, "r/%E0").params, { id: "%E0" }, "an address that does not decode is kept as it is");
  eq(match(routes, "nope"), null);
});

test("the first route that fits wins", () => {
  const both = routesOf({ "r/new": "recipe-new", "r/:id": "recipe-page" });
  eq(match(both, "r/new").page, "recipe-new");
  eq(match(both, "r/8").page, "recipe-page");
});

test("an address is read inside its app", () => {
  eq(within("/a/recipes/", "/a/recipes/r/7"), "r/7");
  eq(within("/a/recipes/", "/a/recipes"), "");
  eq(within("/a/recipes/", "/a/chores/"), null);
  eq(appIdOf("/a/recipes/r/7"), "recipes");
  eq(appIdOf("/a/recipes"), "recipes");
  eq(appIdOf("/"), null);
  eq(appIdOf("/a/Recipes/"), null);
});
