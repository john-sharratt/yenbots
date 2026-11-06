import { listenForChanges, listenForReload, REOPEN_MS } from "/_/framework/feed.js";
import { FakeEventSource, FakePage } from "./fakes.js";
import { eq, test } from "./harness.js";

/** A `setTimeout` the test runs by hand. */
function clock() {
  const due = [];
  return { later: (fn, ms) => due.push([fn, ms]), due };
}

test("a change is heard, and coming back after a drop reads everything again", () => {
  const heard = [];
  const { later } = clock();
  const source = listenForChanges("/api/v1/events/recipes", FakeEventSource, (e) => heard.push(e), later);
  eq(source.url, "/api/v1/events/recipes");
  source.fire("open");
  source.fire("change", { data: '{"coll":"recipes","id":"r1","op":"updated","version":2}' });
  source.fire("error");
  source.fire("open");
  source.fire("reset");
  eq(heard, [{ coll: "recipes", id: "r1", op: "updated", version: 2 }, { reset: true }, { reset: true }]);
});

test("a stream the browser gave up on is opened again, and reads everything again", () => {
  const heard = [];
  const c = clock();
  const before = FakeEventSource.made.length;
  const first = listenForChanges("/api/v1/events/recipes", FakeEventSource, (e) => heard.push(e), c.later);
  first.readyState = 0;
  first.fire("error");
  eq(c.due.length, 0, "the browser reconnects this one itself");
  first.readyState = 2;
  first.fire("error");
  eq(c.due.map(([, ms]) => ms), [REOPEN_MS]);
  c.due[0][0]();
  const second = FakeEventSource.made[FakeEventSource.made.length - 1];
  eq([FakeEventSource.made.length - before, second.url], [2, "/api/v1/events/recipes"]);
  second.fire("open");
  second.fire("change", { data: '{"coll":"recipes","id":"r2","op":"created","version":1}' });
  eq(heard, [{ reset: true }, { coll: "recipes", id: "r2", op: "created", version: 1 }]);
});

test("a page reloads onto a new commit at once, or when it next shows", () => {
  let reloads = 0;
  const page = new FakePage("visible");
  const source = listenForReload(FakeEventSource, page, () => reloads++, clock().later);
  eq(source.url, "/api/v1/events/_reload");
  source.fire("reload");
  eq(reloads, 1);
  page.visibilityState = "hidden";
  source.fire("reload");
  eq(reloads, 1, "not while hidden");
  page.visibilityState = "visible";
  page.fire("visibilitychange");
  page.fire("visibilitychange");
  eq(reloads, 2, "once");
});
