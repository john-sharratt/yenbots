// Every error a page meets, posted to the site's log (§18.1) — counted
// rather than flooding: an error is posted the first time, its repeats only
// counted, and the count posted when the page is put away. A page that goes
// wrong in a loop posts at most `limit` distinct errors.

/** An error, a rejection's reason or a thrown value as `{message, stack}`. */
export function describe(thrown) {
  if (thrown instanceof Error) {
    return { message: `${thrown.name}: ${thrown.message}`, stack: thrown.stack ?? "" };
  }
  if (typeof thrown === "string") return { message: thrown, stack: "" };
  let message;
  try {
    message = JSON.stringify(thrown);
  } catch {
    message = String(thrown);
  }
  return { message: message ?? String(thrown), stack: "" };
}

export class Reporter {
  /**
   * @param app the app the page is
   * @param post sends one report to `POST /api/v1/errors`
   */
  constructor(app, post, { limit = 20 } = {}) {
    this.app = app;
    this.post = post;
    this.limit = limit;
    this.seen = new Map();
    this.posted = 0;
  }

  /** Report `message`; answers whether it was posted now. */
  error(message, { stack = "", page = "" } = {}) {
    const held = this.seen.get(message);
    if (held) {
      held.repeats++;
      return false;
    }
    const sent = this.posted < this.limit;
    this.seen.set(message, { message, stack, page, repeats: 0, sent });
    if (!sent) return false;
    this.posted++;
    this.post({ app: this.app, page, message, stack });
    return true;
  }

  /** Post the repeats counted since the last flush; answers how many reports that took. */
  flush() {
    let posts = 0;
    for (const e of this.seen.values()) {
      if (!e.sent || e.repeats === 0) continue;
      this.post({ app: this.app, page: e.page, message: e.message, stack: e.stack, repeats: e.repeats });
      e.repeats = 0;
      posts++;
    }
    return posts;
  }
}
