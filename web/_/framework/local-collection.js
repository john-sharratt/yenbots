// A collection that lives entirely in the browser's IndexedDB, used in demo
// mode so an unsigned visitor can add, edit, and search records without a
// server. It presents the same surface as the server-backed `Collection`:
// `.value`, `.get`, `.add`, `.update`, `.remove`, `.search`, `.sum`,
// `.loading`, `.error`, `.offline`, `.truncated`, `.mayAdd`, `.mayEdit()`,
// `.reload`.

import { computed, signal } from "../vendor/signals-core.mjs";
import { keyOf, openKeep } from "./idb.js";
import { flatten, merge, ordered, OWN } from "./records.js";

/** Generate a local id for a demo record. */
function localId() {
  return `local-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
}

/**
 * A local collection backed by IndexedDB, with the same surface as the
 * server-backed `Collection`.
 *
 * @param {string} appName - the app id, for the IndexedDB database name
 * @param {string} collName - the collection name
 * @param {object} opts - `{ where, order }` like the server collection
 * @param {Function} compare - the collator function
 * @param {string[]|null} declaredFields - the manifest's declared field names
 *   for this collection, or null if unavailable
 * @param {object|null} store - optional `{load, save}` store; defaults to
 *   IndexedDB via `openKeep`/`keyOf`
 */
export class LocalCollection {
  constructor(appName, collName, { where, order } = {}, compare, declaredFields, store = null) {
    this.name = collName;
    this.where = where ?? null;
    this.order = order ?? null;
    this.compare = compare;
    this._declaredFields = declaredFields ?? null;
    this._store = store ?? keyOf(openKeep(`yenbot-${appName}`), `demo:${collName}`);

    this.docs = signal([]);
    this.loading = signal(false);
    this.error = signal(null);
    this.offline = signal(false);
    this.truncated = signal(false);

    this.records = computed(() => {
      const docs = this.docs.value;
      const filtered = this.where ? docs.filter((d) => matchesWhere(d.data, this.where)) : docs;
      return ordered(filtered, this.order, this.compare).map(flatten);
    });

    // Load from IndexedDB on construction.
    this._load();
  }

  /** The records, in order, each with its fields and its own names. */
  get value() {
    return this.records.value;
  }

  /** The record `id`, or undefined. */
  get(id) {
    return this.value.find((r) => r.id === id);
  }

  /** Whether the caller may add a record. */
  get mayAdd() {
    return true;
  }

  /** Whether the caller may edit `record`. */
  mayEdit() {
    return true;
  }

  /** Reload from IndexedDB. */
  async reload() {
    return this._load();
  }

  async _load() {
    try {
      const stored = await this._store.load();
      this.docs.value = Array.isArray(stored) ? stored : [];
      this.error.value = null;
    } catch (e) {
      this.error.value = e;
    }
    this.loading.value = false;
  }

  async _persist() {
    await this._store.save(this.docs.peek());
  }

  /**
   * Check that the fields in `data` are declared in the manifest.
   * Throws like the site's refusal if a field is not declared.
   */
  _checkFields(data) {
    if (!this._declaredFields) return; // no manifest info, skip the check
    for (const key of Object.keys(data)) {
      if (OWN.includes(key)) {
        throw new Error(`field \`${key}\` is reserved`);
      }
      if (!this._declaredFields.includes(key)) {
        const err = new Error(`field \`${key}\` is not declared in the manifest`);
        err.code = "invalid";
        throw err;
      }
    }
  }

  /** Add a record. */
  async add(fields) {
    this._checkFields(fields);
    const now = Date.now();
    const doc = {
      id: localId(),
      owner: "demo",
      data: { ...fields },
      version: 1,
      created_ms: now,
      updated_ms: now,
      pending: false,
    };
    const docs = this.docs.peek();
    this.docs.value = [...docs, doc];
    await this._persist();
    return flatten(doc);
  }

  /** Update record `id` with `patch`. */
  async update(id, patch) {
    this._checkFields(patch);
    const docs = this.docs.peek();
    const idx = docs.findIndex((d) => d.id === id);
    if (idx < 0) {
      throw new Error(`no such record: ${id}`);
    }
    const updated = {
      ...docs[idx],
      data: merge(docs[idx].data, patch),
      version: docs[idx].version + 1,
      updated_ms: Date.now(),
    };
    this.docs.value = docs.map((d, i) => (i === idx ? updated : d));
    await this._persist();
    return flatten(updated);
  }

  /** Delete record `id`. */
  async remove(id) {
    const docs = this.docs.peek();
    const idx = docs.findIndex((d) => d.id === id);
    if (idx < 0) {
      throw new Error(`no such record: ${id}`);
    }
    this.docs.value = docs.filter((d) => d.id !== id);
    await this._persist();
  }

  /** Full-text search over the records. */
  async search(words, limit = 20) {
    const lower = words.toLowerCase();
    const terms = lower.split(/\s+/).filter(Boolean);
    let results = this.docs.value.filter((d) => {
      const text = Object.values(d.data)
        .filter((v) => typeof v === "string")
        .join(" ")
        .toLowerCase();
      return terms.every((t) => text.includes(t));
    });
    results = results.slice(0, limit);
    return results.map(flatten);
  }

  /** Counts and sums, grouped. */
  async sum({ group, of, where } = {}) {
    const docs = this.docs.value;
    const groups = new Map();
    for (const d of docs) {
      const key = group ? String(d.data[group] ?? "") : "";
      if (!groups.has(key)) {
        groups.set(key, { count: 0, sum: 0, min: null, max: null });
      }
      const g = groups.get(key);
      g.count++;
      if (of) {
        const v = d.data[of];
        if (typeof v === "number") {
          g.sum += v;
          if (g.min === null || v < g.min) g.min = v;
          if (g.max === null || v > g.max) g.max = v;
        }
      }
    }
    return [...groups.entries()].map(([key, g]) => ({
      key,
      count: g.count,
      sum: g.sum,
      average: g.count > 0 ? g.sum / g.count : 0,
      min: g.min,
      max: g.max,
    }));
  }
}

/**
 * A simple `where` matcher matching the server's query language.
 * Supports `field = value`, `field != value`, `field <= value`, `field >= value`,
 * `field < value`, `field > value`, and `and` between clauses.
 */
function matchesWhere(data, where) {
  if (!where) return true;
  const clauses = where.split(/\s+and\s+/i);
  for (const clause of clauses) {
    const m = clause.match(/^\s*(\w+)\s*(=|!=|<=|>=|<|>)\s*(.+?)\s*$/);
    if (!m) return false;
    const [, field, op, rawVal] = m;
    let val;
    if (rawVal === "true") val = true;
    else if (rawVal === "false") val = false;
    else if (/^-?\d+(\.\d+)?$/.test(rawVal)) val = Number(rawVal);
    else val = rawVal.replace(/^['"]|['"]$/g, "");
    const actual = data[field];
    let ok = false;
    switch (op) {
      case "=":
        ok = actual === val;
        break;
      case "!=":
        ok = actual !== val;
        break;
      case "<=":
        ok = actual <= val;
        break;
      case ">=":
        ok = actual >= val;
        break;
      case "<":
        ok = actual < val;
        break;
      case ">":
        ok = actual > val;
        break;
      default:
        ok = false;
    }
    if (!ok) return false;
  }
  return true;
}
