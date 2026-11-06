// The app a page is (§8.2): who is signed in, what the site says the app
// holds, its collections, its pages, and the queue every write goes
// through. `/_/yen.js` makes the one a page has, from the browser; tests
// make their own, from fakes.

import { signal } from "../vendor/signals-core.mjs";
import { ApiError, Offline } from "./api.js";
import { Collection } from "./collection.js";
import { Queue } from "./queue.js";
import { routesOf } from "./router.js";

export class Sdk {
  /**
   * @param id the app's id
   * @param api an `Api` for the app
   * @param storage where the queue keeps its writes
   * @param cache `{load, save}` by key, where collections keep what they last loaded
   * @param compare how text sorts (`Intl.Collator`'s `compare`)
   */
  constructor(id, { api, storage, cache, compare, now }) {
    this.id = id;
    this.api = api;
    this.cache = cache;
    this.compare = compare;
    /** `GET /me`: who is signed in, and whether approved. */
    this.me = signal(null);
    /** `GET /apps/{id}`: the app's collections and their fields. */
    this.info = signal(null);
    /** Why the app cannot be opened — not shared with the caller, or not there. */
    this.closed = signal(null);
    this.routes = signal([]);
    this.title = signal("");
    this.collections = new Map();
    this.queue = new Queue({
      send: (w) => this.send(w),
      storage,
      now,
      landed: (w, answer) => this.each(w.coll, (c) => c.landed(w, answer)),
    });
  }

  /** The app's pages and title (`defineApp`). */
  define({ routes, title }) {
    this.routes.value = routesOf(routes);
    if (title) this.title.value = title;
  }

  /** The collection `name`, filtered and ordered — one per page that asks the same. */
  collection(name, { where, order } = {}) {
    const key = JSON.stringify([name, where ?? null, order ?? null]);
    if (!this.collections.has(key)) this.collections.set(key, new Collection(this, name, { where, order }));
    return this.collections.get(key);
  }

  each(coll, fn) {
    for (const c of this.collections.values()) if (c.name === coll) fn(c);
  }

  /**
   * `GET path`, kept under `key` for a visit with no connection, which is
   * answered what was kept. A cache that cannot be read or written leaves
   * the visit as it would be without one.
   */
  async kept(key, path) {
    try {
      const answer = await this.api.call("GET", path);
      await this.cache.save(key, answer).catch(() => {});
      return answer;
    } catch (e) {
      if (!(e instanceof Offline)) throw e;
      const held = await this.cache.load(key).catch(() => undefined);
      if (held === undefined || held === null) throw e;
      return held;
    }
  }

  /**
   * Learn who is signed in and what the app holds — from what the last
   * visit kept, when the site is out of reach — then send what an earlier
   * visit left. Answers whether the app is open to the caller; it may be
   * called again, after a failure.
   */
  async start() {
    this.closed.value = null;
    // What an earlier visit left is read once; a second start keeps what this one added.
    this.opening ??= this.queue.open();
    await this.opening;
    const me = await this.kept("me", "/me");
    this.me.value = me;
    if (!me.signed_in || me.account.status !== "active") return false;
    try {
      const info = await this.kept("info", `/apps/${this.id}`);
      this.info.value = info;
      if (!this.title.peek()) this.title.value = info.name;
    } catch (e) {
      if (e instanceof ApiError) {
        this.closed.value = e;
        return false;
      }
      throw e;
    }
    this.queue.flush();
    return true;
  }

  /** A change from the feed, or `{reset: true}` to read everything again. */
  heard(event) {
    if (event.reset) {
      for (const c of this.collections.values()) c.reload();
      return;
    }
    this.each(event.coll, (c) => c.heard(event));
  }

  /** One queued write, sent. */
  send(w) {
    const at = `/data/${this.id}/${w.coll}`;
    switch (w.op) {
      case "create":
        return this.api.call("POST", at, w.data);
      case "update":
        return this.api.call("PATCH", `${at}/${w.id}`, w.data, { version: w.version });
      case "delete":
        return this.api.call("DELETE", `${at}/${w.id}`);
      default:
        throw new Error(`no such write as ${w.op}`);
    }
  }
}
