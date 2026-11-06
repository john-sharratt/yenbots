import { Api } from "/_/framework/api.js";
import { memoryStorage } from "/_/framework/queue.js";
import { Sdk } from "/_/framework/sdk.js";
import { effect } from "/_/vendor/signals-core.mjs";
import { fakeFetch } from "./fakes.js";
import { eq, test } from "./harness.js";

const doc = (id, text, version = 1, owner = "1") => ({ id, owner, data: { text }, version, created_ms: Number(id.slice(1)), updated_ms: 0 });
const ME = { signed_in: true, site: "Yen Bots", account: { id: "1", name: "Yen", status: "active" } };
const INFO = {
  id: "home",
  name: "Home",
  collections: {
    notes: { scope: "private", fields: { text: { type: "text", required: true } }, search: [], may_add: true },
    chores: { scope: "shared", fields: { task: { type: "text" } }, search: [], may_add: false },
  },
};

function memoryCache() {
  const held = new Map();
  return { load: async (k) => held.get(k), save: async (k, v) => void held.set(k, v), held };
}

function sdk(answers, cache = memoryCache()) {
  const f = fakeFetch(answers);
  const s = new Sdk("home", {
    api: new Api("home", f.fetch),
    storage: memoryStorage(),
    cache,
    compare: (a, b) => (a < b ? -1 : a > b ? 1 : 0),
    now: () => 50,
  });
  return { s, calls: f.calls, cache };
}

const shown = (c) => c.value.map((r) => [r.id, r.text, r.pending]);

test("a collection loads, shows a write at once as pending, and lands it", async () => {
  const { s, calls } = sdk([
    [200, { docs: [doc("r1", "milk")], after: null }],
    [200, ME],
    [200, INFO],
    [201, doc("r2", "eggs")],
  ]);
  const notes = s.collection("notes");
  eq(s.collection("notes") === notes, true, "one collection per page that asks the same");
  await notes.ready;
  eq(await s.start(), true);
  eq(shown(notes), [["r1", "milk", false]]);
  let renders = 0;
  effect(() => {
    notes.value;
    renders++;
  });
  const adding = notes.add({ text: "eggs" });
  eq(shown(notes), [
    ["r1", "milk", false],
    ["pending-1", "eggs", true],
  ]);
  const added = await adding;
  eq([added.id, added.text], ["r2", "eggs"]);
  eq(shown(notes), [
    ["r1", "milk", false],
    ["r2", "eggs", false],
  ]);
  eq(renders, 3, "loaded, pending, landed — never neither");
  eq(
    calls.map((c) => `${c.method} ${c.url}`),
    ["GET /api/v1/data/home/notes?limit=200", "GET /api/v1/me", "GET /api/v1/apps/home", "POST /api/v1/data/home/notes"],
  );
});

test("a change from the feed is read unless already held at that version", async () => {
  const { s, calls } = sdk([
    [200, { docs: [doc("r1", "milk"), doc("r2", "eggs")], after: null }],
    [200, doc("r1", "oat milk", 2)],
    [404, { error: "not_found", detail: "no such record" }],
  ]);
  const notes = s.collection("notes");
  await notes.ready;
  await notes.heard({ coll: "notes", id: "r1", op: "updated", version: 1 });
  eq(calls.length, 1, "held at that version: not read");
  await notes.heard({ coll: "notes", id: "r1", op: "updated", version: 2 });
  await notes.heard({ coll: "notes", id: "r2", op: "deleted", version: 0 });
  eq(shown(notes), [["r1", "oat milk", false]]);
  await notes.heard({ coll: "notes", id: "r1", op: "updated", version: 3 });
  eq(shown(notes), [], "no longer shared with the caller");
});

test("a reset reads every collection again", async () => {
  const { s, calls } = sdk([
    [200, { docs: [], after: null }],
    [200, { docs: [doc("r5", "back")], after: null }],
  ]);
  const notes = s.collection("notes");
  await notes.ready;
  s.heard({ reset: true });
  await notes.reloading;
  eq(shown(notes), [["r5", "back", false]]);
  eq(calls.length, 2);
});

test("a collection loads every page, and with no connection shows what it last loaded", async () => {
  const online = sdk([
    [200, { docs: [doc("r1", "a")], after: "c1" }],
    [200, { docs: [doc("r2", "b")], after: null }],
  ]);
  const notes = online.s.collection("notes", { order: "-text" });
  await notes.ready;
  eq(online.calls[1].url, "/api/v1/data/home/notes?order=-text&limit=200&after=c1");
  eq(shown(notes), [
    ["r2", "b", false],
    ["r1", "a", false],
  ]);
  const offline = sdk(["offline"], online.cache);
  const kept = offline.s.collection("notes", { order: "-text" });
  await kept.ready;
  eq([kept.offline.value, kept.error.value, shown(kept).length], [true, null, 2]);
});

test("who may add and change is the site's to say", async () => {
  const { s } = sdk([
    [200, { docs: [], after: null }],
    [200, { docs: [], after: null }],
    [200, ME],
    [200, INFO],
  ]);
  const notes = s.collection("notes");
  const chores = s.collection("chores");
  await notes.ready;
  await chores.ready;
  await s.start();
  eq([notes.mayAdd, chores.mayAdd], [true, false]);
  eq([notes.mayEdit({ owner: "1" }), notes.mayEdit({ owner: "2" }), chores.mayEdit({ owner: "2" })], [true, false, false]);
});

test("with no connection the app opens as the last visit left it, and starts again once back", async () => {
  const cache = memoryCache();
  const first = sdk([[200, ME], [200, INFO]], cache);
  eq(await first.s.start(), true);
  const later = sdk(["offline", "offline"], cache);
  eq(await later.s.start(), true);
  eq([later.s.me.value.account.name, later.s.title.value], ["Yen", "Home"]);
  const fresh = sdk(["offline", [200, ME], [200, INFO]]);
  let failed = null;
  await fresh.s.start().catch((e) => (failed = e.name));
  eq(failed, "Offline", "a first visit with no connection has nothing to show");
  eq(await fresh.s.start(), true, "started again");
});

test("an account not yet approved opens nothing", async () => {
  const { s } = sdk([[200, { signed_in: true, account: { id: "2", status: "pending" } }]]);
  eq(await s.start(), false);
  eq(s.info.value, null);
});

test("an app not shared with the caller is closed", async () => {
  const { s } = sdk([
    [200, ME],
    [404, { error: "no_app", detail: "there is no app `home`" }],
  ]);
  eq(await s.start(), false);
  eq(s.closed.value.code, "no_app");
});
