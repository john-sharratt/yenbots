// The toasts on the screen (§8.5): a short message that goes by itself — or
// stays, when it carries an action — at most three at once, newest last;
// when a fourth comes, the oldest without an action makes way, so an
// action is never taken off the screen before it can be used.
// `later(fn, ms)` is the page's `setTimeout`, so the queue can be tested.

import { signal } from "../vendor/signals-core.mjs";

/** How long a toast with no action stays. */
export const SHOWN_MS = 4000;

/** The most on the screen at once; an older one makes way. */
export const MOST = 3;

export class Toasts {
  constructor(later = (fn, ms) => setTimeout(fn, ms)) {
    this.later = later;
    this.shown = signal([]);
    this.next = 1;
  }

  /**
   * Show `message`: `tone` is `info`, `success` or `danger`; `action` is
   * `{label, run}`, which keeps it up until it is used or dismissed. Answers
   * its id.
   */
  show(message, { tone = "info", action = null, ms = SHOWN_MS } = {}) {
    const id = this.next++;
    const all = [...this.shown.peek(), { id, message, tone, action }];
    while (all.length > MOST) {
      // The one just shown is never the one that makes way.
      const plain = all.slice(0, -1).findIndex((t) => !t.action);
      all.splice(plain === -1 ? 0 : plain, 1);
    }
    this.shown.value = all;
    if (!action) this.later(() => this.dismiss(id), ms);
    return id;
  }

  dismiss(id) {
    const left = this.shown.peek().filter((t) => t.id !== id);
    if (left.length !== this.shown.peek().length) this.shown.value = left;
  }
}

/** The page's toasts: `toast("Saved")`, `toast("Couldn't save", {tone: "danger"})`. */
export const toasts = new Toasts();

export const toast = (message, options) => toasts.show(message, options);
