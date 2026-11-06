// Stand-ins for what the browser gives the framework.

/**
 * A `fetch` that records each call and answers from `answers` in turn:
 * `[status, body]` with `body` sent as JSON, `[status, undefined, text]`
 * with `text` sent as it is, or "offline".
 */
export function fakeFetch(answers) {
  const calls = [];
  const fetch = async (url, init) => {
    calls.push({ url, ...init, body: init.body === undefined ? undefined : JSON.parse(init.body) });
    const next = answers.shift();
    if (next === undefined) throw new Error(`no answer left for ${init.method} ${url}`);
    if (next === "offline") throw new TypeError("Failed to fetch");
    const [status, body, raw] = next;
    const text = raw ?? (body === undefined ? "" : JSON.stringify(body));
    return { ok: status < 300, status, text: async () => text };
  };
  return { fetch, calls };
}

/** Something events are dispatched on by name, as an `EventSource` or a document is. */
class Target {
  constructor() {
    this.listeners = {};
  }
  addEventListener(name, fn) {
    (this.listeners[name] ??= []).push(fn);
  }
  fire(name, event = {}) {
    for (const fn of this.listeners[name] ?? []) fn(event);
  }
}

/** An `EventSource` the test drives; `FakeEventSource.made` holds every one made. */
export class FakeEventSource extends Target {
  static made = [];
  constructor(url) {
    super();
    this.url = url;
    this.readyState = 0;
    FakeEventSource.made.push(this);
  }
}

/** A document whose visibility the test sets. */
export class FakePage extends Target {
  constructor(visibilityState) {
    super();
    this.visibilityState = visibilityState;
  }
}
