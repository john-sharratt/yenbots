// A record as a page sees it: its fields, and beside them the record's own
// names — `id`, `owner`, `version`, `created_ms`, `updated_ms`, and
// `pending` while a write of it waits to be sent. The manifest refuses a
// field named any of these (`src/apps/manifest.rs`, `RESERVED`), so the two
// never collide.

export const OWN = ["id", "owner", "version", "created_ms", "updated_ms", "pending"];

/** The API's `{id, owner, data, version, …}` as a page's record. */
export function flatten(doc) {
  return {
    ...doc.data,
    id: doc.id,
    owner: doc.owner,
    version: doc.version,
    created_ms: doc.created_ms,
    updated_ms: doc.updated_ms,
    pending: doc.pending === true,
  };
}

/** A record's fields alone, without its own names. */
export function fieldsOf(record) {
  const out = {};
  for (const [k, v] of Object.entries(record)) {
    if (!OWN.includes(k)) out[k] = v;
  }
  return out;
}

/** `patch` merged into `data` as the server merges it: a field set to null is removed. */
export function merge(data, patch) {
  const out = { ...data };
  for (const [k, v] of Object.entries(patch)) {
    if (v === null) delete out[k];
    else out[k] = v;
  }
  return out;
}

/**
 * `docs` with the queued writes to `coll` laid over them, in the order they
 * were made: a create is added, an update merged, a deletion removed. Each
 * record a waiting write touches is marked `pending`.
 */
export function overlay(docs, writes, coll, me) {
  let out = docs.slice();
  for (const w of writes) {
    if (w.coll !== coll) continue;
    if (w.op === "create") {
      out.push({
        id: w.id,
        owner: me,
        data: w.data,
        version: 0,
        created_ms: w.at,
        updated_ms: w.at,
        pending: true,
      });
    } else if (w.op === "update") {
      out = out.map((d) => (d.id === w.id ? { ...d, data: merge(d.data, w.data), pending: true } : d));
    } else if (w.op === "delete") {
      out = out.filter((d) => d.id !== w.id);
    }
  }
  return out;
}

/** Compare two field values as the store orders them: nothing first, then numbers, then text. */
function compareValues(a, b, compareText) {
  const absent = (v) => v === undefined || v === null;
  if (absent(a) || absent(b)) return absent(a) === absent(b) ? 0 : absent(a) ? -1 : 1;
  if (typeof a === "number" && typeof b === "number") return a - b;
  if (typeof a === "number") return -1;
  if (typeof b === "number") return 1;
  return compareText(String(a), String(b));
}

/**
 * `docs` in `order` — a field, `-field` for descending, creation order when
 * absent — as the API answers a page of them, so a record added or changed
 * on the page lands where a reload would put it. Ties fall to creation order,
 * newest first when descending.
 */
export function ordered(docs, order, compareText) {
  const desc = typeof order === "string" && order.startsWith("-");
  const field = order ? (desc ? order.slice(1) : order) : null;
  const sign = desc ? -1 : 1;
  const byAge = (a, b) => a.created_ms - b.created_ms || (a.id < b.id ? -1 : a.id > b.id ? 1 : 0);
  return docs.slice().sort((a, b) => {
    const by = field ? compareValues(a.data[field], b.data[field], compareText) : 0;
    return sign * (by || byAge(a, b));
  });
}
