import { LocalCollection } from "/_/framework/local-collection.js";
import { eq, test } from "./harness.js";

/** A simple in-memory store for tests. */
function memStore() {
  let data = null;
  return {
    load: async () => data,
    save: async (v) => { data = JSON.parse(JSON.stringify(v)); },
  };
}

const compare = (a, b) => (a < b ? -1 : a > b ? 1 : 0);

const fields = ["title", "serves", "minutes"];

function makeColl(opts = {}) {
  return new LocalCollection("test", "recipes", opts, compare, fields, memStore());
}

test("add and read back", async () => {
  const c = makeColl();
  await c.reload();
  const r = await c.add({ title: "Pancakes", serves: 2, minutes: 15 });
  eq(r.title, "Pancakes", "title");
  eq(r.owner, "demo", "owner");
  eq(r.version, 1, "version");
  eq(c.value.length, 1, "one record");
  eq(c.value[0].title, "Pancakes", "value reads back");
});

test("update changes fields and bumps version", async () => {
  const c = makeColl();
  await c.reload();
  const r = await c.add({ title: "Soup", serves: 4, minutes: 30 });
  const updated = await c.update(r.id, { serves: 6 });
  eq(updated.serves, 6, "serves updated");
  eq(updated.version, 2, "version bumped");
  eq(c.get(r.id).title, "Soup", "other fields kept");
});

test("remove deletes the record", async () => {
  const c = makeColl();
  await c.reload();
  const r = await c.add({ title: "Gone", serves: 1, minutes: 5 });
  eq(c.value.length, 1, "one before remove");
  await c.remove(r.id);
  eq(c.value.length, 0, "none after remove");
});

test("order sorts by field", async () => {
  const c = makeColl({ order: "minutes" });
  await c.reload();
  await c.add({ title: "Slow", serves: 1, minutes: 60 });
  await c.add({ title: "Fast", serves: 1, minutes: 5 });
  await c.add({ title: "Medium", serves: 1, minutes: 20 });
  const titles = c.value.map((r) => r.title);
  eq(titles, ["Fast", "Medium", "Slow"], "ascending by minutes");
});

test("order descending", async () => {
  const c = makeColl({ order: "-minutes" });
  await c.reload();
  await c.add({ title: "Slow", serves: 1, minutes: 60 });
  await c.add({ title: "Fast", serves: 1, minutes: 5 });
  const titles = c.value.map((r) => r.title);
  eq(titles, ["Slow", "Fast"], "descending by minutes");
});

test("where filters records", async () => {
  const c = makeColl({ where: "minutes <= 30" });
  await c.reload();
  await c.add({ title: "Quick", serves: 1, minutes: 10 });
  await c.add({ title: "Slow", serves: 1, minutes: 90 });
  await c.add({ title: "Medium", serves: 1, minutes: 30 });
  eq(c.value.length, 2, "two records match");
  const titles = c.value.map((r) => r.title);
  eq(titles, ["Quick", "Medium"], "filtered titles");
});

test("search finds by text", async () => {
  const c = makeColl();
  await c.reload();
  await c.add({ title: "Pancakes", serves: 2, minutes: 15 });
  await c.add({ title: "Omelette", serves: 1, minutes: 10 });
  const results = await c.search("pancake");
  eq(results.length, 1, "one result");
  eq(results[0].title, "Pancakes", "found the right one");
});

test("mayAdd is true and mayEdit is true", async () => {
  const c = makeColl();
  await c.reload();
  eq(c.mayAdd, true, "mayAdd");
  eq(c.mayEdit(), true, "mayEdit");
});

test("undeclared field throws", async () => {
  const c = makeColl();
  await c.reload();
  let threw = false;
  try {
    await c.add({ bogus: 1 });
  } catch (e) {
    threw = true;
    eq(e.code, "invalid", "error code");
  }
  eq(threw, true, "should have thrown");
});

test("sum groups and counts", async () => {
  const c = makeColl();
  await c.reload();
  await c.add({ title: "A", serves: 2, minutes: 10 });
  await c.add({ title: "B", serves: 4, minutes: 20 });
  const groups = await c.sum({ group: "serves", of: "minutes" });
  const two = groups.find((g) => g.key === "2");
  eq(two.count, 1, "count for serves=2");
  eq(two.sum, 10, "sum of minutes for serves=2");
});
