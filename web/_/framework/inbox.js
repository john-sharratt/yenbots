// `app.inbox` (§11.2): the account's notifications, how many are unread, and
// the apps it muted — what the bell shows. It is read when a page opens,
// when the service worker says a push arrived, and when the page shows
// again after being hidden.

import { signal } from "../vendor/signals-core.mjs";

export class Inbox {
  constructor(api) {
    this.api = api;
    this.items = signal([]);
    this.unread = signal(0);
    this.muted = signal([]);
  }

  /** Read the inbox again. */
  async load() {
    const answer = await this.api.call("GET", "/inbox");
    this.items.value = answer.items;
    this.unread.value = answer.unread;
    this.muted.value = answer.muted;
    return answer;
  }

  /** Mark notification `id` read — at once on the page, then on the site. */
  async read(id) {
    const item = this.items.peek().find((i) => i.id === id);
    if (!item || item.read_ms !== null) return;
    this.items.value = this.items.peek().map((i) => (i.id === id ? { ...i, read_ms: Date.now() } : i));
    this.unread.value = Math.max(0, this.unread.peek() - 1);
    await this.api.call("POST", `/inbox/${encodeURIComponent(id)}/read`);
  }

  /** Mark every notification read. */
  async readAll() {
    const now = Date.now();
    this.items.value = this.items.peek().map((i) => (i.read_ms === null ? { ...i, read_ms: now } : i));
    this.unread.value = 0;
    await this.api.call("POST", "/inbox/all/read");
  }
}
