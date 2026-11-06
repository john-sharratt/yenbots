import { ApiError, Offline } from "/_/framework/api.js";
import { memoryStorage, Queue, rebase } from "/_/framework/queue.js";
import { eq, rejection, settle, test } from "./harness.js";

/**
 * A queue whose `send` answers from `answers` in turn: a record, an
 * `ApiError`, or "offline". `sent` holds each write as it was sent.
 */
function queue(answers, storage = memoryStorage()) {
  const sent = [];
  const landed = [];
  let version = 10;
  const send = async (w) => {
    sent.push({ op: w.op, id: w.id, version: w.version });
    const next = answers.shift();
    if (next === "offline") throw new Offline(new TypeError("Failed to fetch"));
    if (next instanceof ApiError) throw next;
    version++;
    return { id: w.op === "create" ? `r${version}` : w.id, version, data: w.data };
  };
  const q = new Queue({ send, storage, now: () => 5, landed: (w, a) => landed.push([w.op, a.id]) });
  return { q, sent, landed, storage };
}

const stale = (held) => new ApiError(412, { error: "stale", detail: "the record changed since it was read", version: held });
const invalid = () => new ApiError(422, { error: "invalid", detail: "title is required", field: "title" });

test("a write is sent and answers what the site made of it", async () => {
  const { q, sent, landed } = queue([undefined]);
  const doc = await q.push({ op: "create", coll: "notes", data: { t: "a" } });
  eq(doc.id, "r11");
  eq(sent, [{ op: "create", id: "pending-1", version: 0 }]);
  eq(landed, [["create", "r11"]]);
  eq(q.writes.value, []);
});

test("writes made offline wait, in order, and go when the connection is back", async () => {
  const { q, sent } = queue(["offline", undefined, undefined]);
  const first = q.push({ op: "create", coll: "notes", data: { t: "a" } });
  eq(await first, null, "the page learns it waits");
  eq(q.writes.value.length, 1, "kept");
  await q.push({ op: "update", coll: "notes", id: "pending-1", data: { t: "b" }, version: 0 });
  await settle();
  eq(sent, [
    { op: "create", id: "pending-1", version: 0 },
    { op: "create", id: "pending-1", version: 0 },
    { op: "update", id: "r11", version: 11 },
  ]);
  eq(q.writes.value, []);
});

test("a write the page waits on is refused to the page, not held", async () => {
  const { q } = queue([invalid()]);
  const e = await rejection(q.push({ op: "create", coll: "notes", data: {} }));
  eq([e.code, e.field], ["invalid", "title"]);
  eq([q.writes.value, q.held.value], [[], []]);
});

test("a write refused after its page moved on is held, to keep or drop", async () => {
  const storage = memoryStorage();
  const before = queue(["offline"], storage);
  eq(await before.q.push({ op: "update", coll: "notes", id: "r1", data: { t: "mine" }, version: 3 }), null);

  // The next visit sends it; someone else changed the record meanwhile.
  const after = queue([stale(4), undefined], storage);
  await after.q.open();
  eq(after.q.writes.value.length, 1, "the write survived the visit");
  await after.q.flush();
  const [item] = after.q.held.value;
  const why = { status: 412, code: "stale", detail: "the record changed since it was read", field: null, version: 4 };
  eq([item.write.id, item.why], ["r1", why]);
  eq(after.q.keepable(item), true);
  const kept = await after.q.resend(item);
  eq(after.sent[1], { op: "update", id: "r1", version: 4 }, "sent again over the site's version");
  eq([kept.version, after.q.held.value], [11, []]);
});

test("a write sent while signed out waits, rather than being refused", async () => {
  const signedOut = new ApiError(401, { error: "unauthorized", detail: "sign in first" });
  const { q, sent } = queue([signedOut, undefined]);
  eq(await q.push({ op: "create", coll: "notes", data: { t: "a" } }), null);
  eq([q.writes.value.length, q.held.value], [1, []]);
  await q.flush();
  eq([sent.length, q.writes.value], [2, []]);
});

test("a held write that cannot be kept is only dropped", async () => {
  const storage = memoryStorage();
  await queue(["offline"], storage).q.push({ op: "create", coll: "notes", data: {} });
  const { q } = queue([invalid()], storage);
  await q.open();
  await q.flush();
  const [item] = q.held.value;
  eq(q.keepable(item), false);
  eq((await rejection(q.resend(item))).message, "a write refused as invalid cannot be sent again");
  await q.drop(item);
  eq(q.held.value, []);
  eq((await storage.load()).held, [], "dropped for good");
});

test("later writes are rebased on what an earlier one became", () => {
  const rest = [
    { op: "update", coll: "notes", id: "pending-1", version: 0 },
    { op: "update", coll: "notes", id: "r9", version: 2 },
    { op: "create", coll: "notes", id: "pending-3", version: 0 },
  ];
  eq(rebase(rest, { op: "create", coll: "notes", id: "pending-1", version: 0 }, { id: "r7", version: 1 }), [
    { op: "update", coll: "notes", id: "r7", version: 1 },
    { op: "update", coll: "notes", id: "r9", version: 2 },
    { op: "create", coll: "notes", id: "pending-3", version: 0 },
  ]);
  const two = [{ op: "update", coll: "notes", id: "r9", version: 2 }];
  eq(rebase(two, { op: "update", coll: "notes", id: "r9", version: 2 }, { id: "r9", version: 3 })[0].version, 3);
  eq(rebase(two, { op: "update", coll: "notes", id: "r9", version: 1 }, { id: "r9", version: 2 })[0].version, 2, "made against another version: untouched");
  eq(rebase(two, { op: "delete", coll: "notes", id: "r9" }, { deleted: "r9" }), two);
});
