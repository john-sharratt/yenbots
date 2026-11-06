import { Api, ApiError, nfc, Offline, qs } from "/_/framework/api.js";
import { fakeFetch } from "./fakes.js";
import { eq, rejection, test } from "./harness.js";

test("a call sends the app, the version and NFC text", async () => {
  const f = fakeFetch([[200, { id: "1", version: 4 }]]);
  const api = new Api("recipes", f.fetch);
  const answer = await api.call("PATCH", "/data/recipes/recipes/1", { title: "Phở" }, { version: 3 });
  eq(answer, { id: "1", version: 4 });
  eq(f.calls, [
    {
      url: "/api/v1/data/recipes/recipes/1",
      method: "PATCH",
      credentials: "same-origin",
      headers: {
        "X-Yen-App": "recipes",
        Accept: "application/json",
        "Content-Type": "application/json",
        "If-Match": '"3"',
      },
      body: { title: "Phở" },
    },
  ]);
});

test("a refusal is thrown with the site's reason; only a fault is reported", async () => {
  const failed = [];
  const f = fakeFetch([
    [422, { error: "invalid", detail: "title is required", field: "title", reason: "is required" }],
    [412, { error: "stale", detail: "the record changed since it was read", version: 5 }],
    [500, { error: "internal", detail: "disk full" }],
    [418, undefined, "teapot"],
  ]);
  const api = new Api("recipes", f.fetch, (method, path, e) => failed.push([method, path, e.status, e.code]));
  const invalid = await rejection(api.call("POST", "/data/recipes/recipes", {}));
  eq([invalid instanceof ApiError, invalid.status, invalid.field, invalid.reason], [true, 422, "title", "is required"]);
  eq((await rejection(api.call("PATCH", "/x", {}))).version, 5);
  eq((await rejection(api.call("GET", "/y"))).message, "disk full");
  eq((await rejection(api.call("GET", "/z"))).code, "http_418");
  eq(failed, [
    ["GET", "/y", 500, "internal"],
    ["GET", "/z", 418, "http_418"],
  ]);
});

test("the site out of reach is Offline, not a refusal", async () => {
  const failed = [];
  const f = fakeFetch(["offline", [502, undefined, "Bad Gateway"], [503, undefined], [200, undefined, "<html>Sign in to the Wi-Fi</html>"]]);
  const api = new Api("recipes", f.fetch, (...a) => failed.push(a));
  for (let i = 0; i < 4; i++) eq((await rejection(api.call("GET", "/me"))) instanceof Offline, true, `answer ${i}`);
  eq(failed, [], "nothing to report");
});

test("a query string leaves out what is absent", () => {
  eq(qs({ where: "serves >= 4", order: null, limit: 50, after: undefined }), "?where=serves%20%3E%3D%204&limit=50");
  eq(qs({}), "");
});

test("keys are normalised too", () => {
  eq(nfc({ ["Phở"]: ["ơ"] }), { Phở: ["ơ"] });
});
