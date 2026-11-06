// Every write a page makes, in order (§8.6). A write is kept in the
// browser's storage before it is sent, so one made with no connection is
// not lost: it shows as pending, and is sent when the connection is back —
// in the order it was made, each with the version it was made against.
//
// A write the site refuses while its page still waits on it — a form's
// mistake — is the page's to show, and is dropped. One refused later — a
// change sent when the connection came back, to a record someone else
// changed meanwhile — is held for the person to keep or drop, never
// silently overwritten.

import { batch, signal } from "../vendor/signals-core.mjs";
import { Offline } from "./api.js";

/** Storage held in memory: what a queue keeps when the browser keeps nothing. */
export function memoryStorage() {
  let held = null;
  return {
    load: async () => held,
    save: async (value) => {
      held = JSON.parse(JSON.stringify(value));
    },
  };
}

/**
 * The writes after `landed`, rebased on what it became: a change made after
 * it, against the version it was made against, is now made against the
 * version it produced — and one to a record it created names the record's id.
 */
export function rebase(rest, landed, doc) {
  if (landed.op === "delete" || !doc) return rest;
  return rest.map((w) => {
    if (w.op === "create" || w.coll !== landed.coll || w.id !== landed.id) return w;
    const version = w.version === landed.version ? doc.version : w.version;
    return { ...w, id: doc.id, version };
  });
}

export class Queue {
  /**
   * @param send sends one write, answering the record it made or changed
   * @param storage `{load, save}` of `{writes, held, next}`
   * @param landed told `(write, answer)` of each write the site took
   */
  constructor({ send, storage, now = () => Date.now(), landed = () => {} }) {
    this.send = send;
    this.storage = storage;
    this.now = now;
    this.landed = landed;
    /** The writes not yet sent, oldest first. */
    this.writes = signal([]);
    /** The writes the site refused after their page had moved on: `{write, why}`. */
    this.held = signal([]);
    /** Why this browser cannot keep writes, when it cannot. */
    this.fault = signal(null);
    this.next = 1;
    this.waiting = new Map();
    this.draining = null;
  }

  /** Read what an earlier visit left. */
  async open() {
    try {
      const kept = await this.storage.load();
      if (kept) {
        this.writes.value = kept.writes ?? [];
        this.held.value = kept.held ?? [];
        this.next = kept.next ?? 1;
      }
    } catch (e) {
      this.fault.value = e;
    }
  }

  async keep() {
    try {
      await this.storage.save({ writes: this.writes.peek(), held: this.held.peek(), next: this.next });
    } catch (e) {
      this.fault.value = e;
    }
  }

  /**
   * Queue `write` and send it: answers what the site made of it, `null`
   * when it waits for the connection, and throws the site's refusal.
   */
  async push(write) {
    const seq = this.next++;
    const w = { ...write, seq, at: this.now() };
    if (w.op === "create") {
      w.id = `pending-${seq}`;
      w.version = 0;
    }
    this.writes.value = [...this.writes.peek(), w];
    await this.keep();
    const answer = new Promise((resolve, reject) => this.waiting.set(seq, { resolve, reject }));
    this.flush();
    return answer;
  }

  /** Send what waits, in order, until it is all sent or the connection is gone. */
  flush() {
    if (!this.draining) {
      this.draining = this.drain().then(
        (connected) => {
          this.draining = null;
          // A write queued as the last one landed is sent too.
          if (connected && this.writes.peek().length > 0) return this.flush();
        },
        (e) => {
          this.draining = null;
          throw e;
        },
      );
    }
    return this.draining;
  }

  /** Answers whether the connection held to the end. */
  async drain() {
    while (this.writes.peek().length > 0) {
      const w = this.writes.peek()[0];
      const waiter = this.waiting.get(w.seq);
      this.waiting.delete(w.seq);
      let answer;
      try {
        answer = await this.send(w);
      } catch (e) {
        if (waiter) this.waiting.set(w.seq, waiter);
        // Out of reach, or signed out: the write waits for the person to be back.
        if (e instanceof Offline || e.status === 401) {
          // Those still waiting learn their writes are kept for later.
          for (const { resolve } of this.waiting.values()) resolve(null);
          this.waiting.clear();
          return false;
        }
        this.writes.value = this.writes.peek().slice(1);
        this.waiting.delete(w.seq);
        if (waiter) {
          waiter.reject(e);
        } else {
          const why = {
            status: e.status ?? null,
            code: e.code ?? null,
            detail: e.detail ?? e.message,
            field: e.field ?? null,
            version: e.version ?? null,
          };
          this.held.value = [...this.held.peek(), { write: w, why }];
        }
        await this.keep();
        continue;
      }
      // In one step, so the record is never shown neither pending nor landed.
      batch(() => {
        this.landed(w, answer);
        this.writes.value = rebase(this.writes.peek().slice(1), w, answer);
      });
      await this.keep();
      if (waiter) waiter.resolve(answer);
    }
    return true;
  }

  /** Whether a held write may be sent again: one refused for being stale. */
  keepable(item) {
    return item.why.code === "stale";
  }

  /** Send a held write again, over the version the site now holds. */
  async resend(item) {
    if (!this.keepable(item)) throw new Error(`a write refused as ${item.why.code} cannot be sent again`);
    this.held.value = this.held.peek().filter((h) => h !== item);
    const { seq: _seq, at: _at, ...write } = item.write;
    return this.push({ ...write, version: item.why.version });
  }

  /** Let a held write go: the record stays as the site has it. */
  async drop(item) {
    this.held.value = this.held.peek().filter((h) => h !== item);
    await this.keep();
  }
}
