// One collection of an app's records as a signal (§8.2, §8.3): a page that
// reads `recipes.value` re-renders when a recipe changes — on this page, on
// another person's phone through the change feed, or when a write made
// offline is sent. Its writes go through the queue, so they show at once,
// marked pending, and survive a lost connection. With no connection it
// shows what it last loaded.

import { batch, computed, signal } from "../vendor/signals-core.mjs";
import { ApiError, Offline, qs } from "./api.js";
import { flatten, ordered, overlay } from "./records.js";

/** How many pages of 200 a collection loads: two thousand records. */
export const MAX_PAGES = 10;

export class Collection {
  /**
   * @param sdk the app's SDK: its `api`, `queue`, `me`, `info`, `cache` and `compare`
   * @param name the collection, as the manifest names it
   * @param where a query, in the API's `where` language, that the records shown must fit
   * @param order a field, `-field` for descending; creation order when absent
   */
  constructor(sdk, name, { where, order } = {}) {
    this.sdk = sdk;
    this.name = name;
    this.where = where ?? null;
    this.order = order ?? null;
    this.docs = signal([]);
    this.loading = signal(true);
    this.error = signal(null);
    /** Whether what shows is what was last loaded, the site out of reach. */
    this.offline = signal(false);
    /** Whether there were more records than a collection loads. */
    this.truncated = signal(false);
    this.records = computed(() => {
      const me = sdk.me.value?.account?.id ?? null;
      const laid = overlay(this.docs.value, sdk.queue.writes.value, name, me);
      return ordered(laid, this.order, sdk.compare).map(flatten);
    });
    this.reloading = null;
    this.reloadAgain = false;
    this.ready = this.reload();
  }

  /** The records, in order, each with its fields and its own names. */
  get value() {
    return this.records.value;
  }

  /** The record `id`, or undefined. */
  get(id) {
    return this.value.find((r) => r.id === id);
  }

  /** The collection as the site describes it: its scope, its fields and whether the caller may add. */
  get described() {
    return this.sdk.info.value?.collections?.[this.name] ?? null;
  }

  /** Whether the caller may add a record. */
  get mayAdd() {
    return this.described?.may_add === true;
  }

  /** Whether the caller may change `record`: theirs, or in a shared collection they may write. */
  mayEdit(record) {
    const me = this.sdk.me.value?.account?.id;
    const d = this.described;
    return record.owner === me || (d?.scope === "shared" && d.may_add === true);
  }

  get key() {
    return `docs/${this.name}${qs({ where: this.where, order: this.order })}`;
  }

  /** Read the collection again; a reload asked for during one runs once it ends. */
  reload() {
    if (this.reloading) {
      this.reloadAgain = true;
      return this.reloading;
    }
    this.reloading = this.load().finally(() => {
      this.reloading = null;
      if (this.reloadAgain) {
        this.reloadAgain = false;
        this.reload();
      }
    });
    return this.reloading;
  }

  async load() {
    this.loading.value = true;
    try {
      const docs = [];
      let after = null;
      let pages = 0;
      do {
        const q = qs({ where: this.where, order: this.order, limit: 200, after });
        const page = await this.sdk.api.call("GET", `/data/${this.sdk.id}/${this.name}${q}`);
        docs.push(...page.docs);
        after = page.after;
        pages++;
      } while (after && pages < MAX_PAGES);
      batch(() => {
        this.docs.value = docs;
        this.error.value = null;
        this.offline.value = false;
        this.truncated.value = Boolean(after);
      });
      // A cache that cannot be written only leaves the next offline visit without it.
      await this.sdk.cache.save(this.key, docs).catch(() => {});
    } catch (e) {
      if (!(e instanceof Offline)) {
        this.error.value = e;
      } else {
        const held = await this.sdk.cache.load(this.key).catch(() => undefined);
        batch(() => {
          if (held) this.docs.value = held;
          this.offline.value = true;
        });
      }
    } finally {
      this.loading.value = false;
    }
  }

  /** `doc` put where it belongs: in place of the version held, or added. */
  upsert(doc) {
    const docs = this.docs.peek();
    const at = docs.findIndex((d) => d.id === doc.id);
    if (at < 0) this.docs.value = [...docs, doc];
    else if (docs[at].version <= doc.version) this.docs.value = docs.map((d, i) => (i === at ? doc : d));
  }

  without(id) {
    this.docs.value = this.docs.peek().filter((d) => d.id !== id);
  }

  /** A write of this collection the site took. */
  landed(write, answer) {
    if (write.op === "delete") this.without(write.id);
    // A filtered collection cannot tell whether the record still fits.
    else if (this.where) this.reload();
    else this.upsert(answer);
  }

  /** A change the feed told of: read the record unless it is already held at that version. */
  async heard(change) {
    if (this.where) return this.reload();
    if (change.op === "deleted") return this.without(change.id);
    const held = this.docs.peek().find((d) => d.id === change.id);
    if (held && held.version >= change.version) return;
    try {
      this.upsert(await this.sdk.api.call("GET", `/data/${this.sdk.id}/${this.name}/${change.id}`));
    } catch (e) {
      // One no longer shared with the caller is one they no longer see.
      if (e instanceof ApiError && e.code === "not_found") this.without(change.id);
      else if (!(e instanceof Offline)) this.error.value = e;
    }
  }

  /** Add a record: answers it, or null when it waits for the connection. */
  async add(fields) {
    const doc = await this.sdk.queue.push({ op: "create", coll: this.name, data: fields });
    return doc ? flatten(doc) : null;
  }

  /** Change record `id`: answers it, or null when it waits for the connection. */
  async update(id, patch) {
    const version = this.get(id)?.version;
    const doc = await this.sdk.queue.push({ op: "update", coll: this.name, id, data: patch, version });
    return doc ? flatten(doc) : null;
  }

  /** Delete record `id`. */
  async remove(id) {
    const version = this.get(id)?.version;
    await this.sdk.queue.push({ op: "delete", coll: this.name, id, version });
  }

  /** Full-text search, best first; it needs the connection. */
  async search(words, limit = 20) {
    const found = await this.sdk.api.call("GET", `/data/${this.sdk.id}/${this.name}/_search${qs({ q: words, limit })}`);
    return found.docs.map(flatten);
  }

  /** Counts and sums, grouped (`{group, of, where}`); it needs the connection. */
  async sum({ group, of, where } = {}) {
    const q = qs({ group, of, where: where ?? this.where });
    return (await this.sdk.api.call("GET", `/data/${this.sdk.id}/${this.name}/_sum${q}`)).groups;
  }
}
