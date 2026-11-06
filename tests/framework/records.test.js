import { fieldsOf, flatten, merge, ordered, overlay } from "/_/framework/records.js";
import { eq, test } from "./harness.js";

const doc = (id, data, extra = {}) => ({ id, owner: "yen", data, version: 1, created_ms: Number(id), updated_ms: 0, ...extra });
const ids = (docs) => docs.map((d) => d.id);
const compare = (a, b) => (a < b ? -1 : a > b ? 1 : 0);

test("a record is its fields beside its own names", () => {
  const r = flatten(doc("1", { title: "Phở" }));
  eq(r, { title: "Phở", id: "1", owner: "yen", version: 1, created_ms: 1, updated_ms: 0, pending: false });
  eq(fieldsOf(r), { title: "Phở" });
});

test("a merge removes what is set to null, as the server's does", () => {
  eq(merge({ a: 1, b: 2 }, { b: null, c: 3 }), { a: 1, c: 3 });
});

test("queued writes lay over the records, each marked pending", () => {
  const docs = [doc("1", { t: "a" }), doc("2", { t: "b" })];
  const writes = [
    { op: "update", coll: "notes", id: "1", data: { t: "A" } },
    { op: "delete", coll: "notes", id: "2" },
    { op: "create", coll: "notes", id: "pending-3", data: { t: "c" }, at: 9 },
    { op: "create", coll: "other", id: "pending-4", data: {}, at: 9 },
  ];
  const laid = overlay(docs, writes, "notes", "mai");
  eq(ids(laid), ["1", "pending-3"]);
  eq([laid[0].data, laid[0].pending], [{ t: "A" }, true]);
  eq([laid[1].owner, laid[1].version, laid[1].pending], ["mai", 0, true]);
  eq(docs[0].data, { t: "a" }, "the records themselves are untouched");
});

test("records order as the store orders them", () => {
  const docs = [doc("3", { serves: 4 }), doc("1", { serves: 2 }), doc("2", { serves: 4 }), doc("4", {})];
  eq(ids(ordered(docs, null, compare)), ["1", "2", "3", "4"], "creation order");
  eq(ids(ordered(docs, "serves", compare)), ["4", "1", "2", "3"], "nothing first, ties oldest first");
  eq(ids(ordered(docs, "-serves", compare)), ["3", "2", "1", "4"], "descending, ties newest first");
  const named = [doc("1", { t: "b" }), doc("2", { t: "a" })];
  eq(ids(ordered(named, "t", compare)), ["2", "1"]);
});
