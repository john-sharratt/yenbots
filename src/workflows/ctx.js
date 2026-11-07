// The `ctx` a workflow is handed (candle/docs/yenbot_app_host.md §10.2):
// everything a workflow can reach, and nothing else — there is no
// filesystem, process or network in the engine. Each call goes to the
// platform through one host function, `__yen(op, args)`, which answers
// `{ok}` or `{error, detail}`; an error is thrown here as an `Error` with
// its `code` (and `field`, for a write a collection refuses).
//
// The engine sets `__yen` and `__yen_run` (the app, the job and the account
// the run is for) before this module loads, and it takes them off the
// global object, so a workflow reaches the platform through `ctx` alone.

const host = globalThis.__yen;
const run = JSON.parse(globalThis.__yen_run);
delete globalThis.__yen;
delete globalThis.__yen_run;

function call(op, ...args) {
  const answer = JSON.parse(host(op, JSON.stringify(args)));
  if (answer.error !== undefined) {
    const e = new Error(answer.detail);
    e.code = answer.error;
    if (answer.field !== undefined) e.field = answer.field;
    throw e;
  }
  return answer.ok;
}

/** One of the app's collections, read and written as the run's account or the app. */
class Collection {
  constructor(name) {
    this.name = name;
  }

  /** A page of records: `{where, order, limit, after}` as the data route takes them. */
  async list(query = {}) {
    return call("list", this.name, query);
  }

  async get(id) {
    return call("get", this.name, id);
  }

  async create(fields) {
    return call("create", this.name, fields);
  }

  /** Merge `fields` in; `{version}` refuses the write if the record moved on. */
  async update(id, fields, { version } = {}) {
    return call("update", this.name, id, fields, version ?? null);
  }

  async delete(id) {
    return call("delete", this.name, id);
  }

  /** Counts and sums: `{where, group, of}`. */
  async sum(query = {}) {
    return call("sum", this.name, query);
  }

  async search(q, { limit } = {}) {
    return call("search", this.name, q, limit ?? null);
  }
}

const text = (part) => (typeof part === "string" ? part : JSON.stringify(part));

export const ctx = Object.freeze({
  /** The app's id. */
  app: run.app,
  /** The job being run — `{id, due_ms}` — or null for a run asked for at once. */
  job: run.job,
  /** The account the run acts for, or null when it acts as the app. */
  account: run.account,
  data: Object.freeze({ collection: (name) => new Collection(name) }),
  /**
   * A notification (§11) to `to` — an account's id, `group:<name>`, or
   * `admins` — `{title, body, open}`: written to each one's inbox and pushed
   * to their devices. Answers `{accounts, devices}`. The app's `app.yaml`
   * says `notify: true`.
   */
  async notify(to, message) {
    return call("notify", to, message);
  },
  /** A job for later: `workflow` run with `args` at `at` (milliseconds, or a Date). */
  async schedule(at, workflow, args = {}) {
    const when = at instanceof Date ? at.getTime() : at;
    return call("schedule", when, workflow, args);
  },
  /** Another of the app's workflows, run in this run and awaited. */
  async run(workflow, args = {}) {
    if (!call("has", workflow)) throw new Error(`there is no workflow ${workflow}`);
    const module = await import(`${run.dir}${workflow}.js`);
    return module.default(ctx, args);
  },
  /** `fn` run once for `key`, whatever retries come: its answer is kept and given again. */
  async once(key, fn) {
    const held = call("once_get", String(key));
    if (held !== null) return held.value;
    const value = await fn();
    call("once_put", String(key), value === undefined ? null : value);
    return value;
  },
  /** `ms` as Yen reads it, in her time zone: "Sunday 8 November, 7:30 pm". */
  time: (ms) => call("time", ms),
  now: () => call("now"),
  /** A line in the run's log. */
  log: (...parts) => {
    call("log", parts.map(text).join(" "));
  },
});
