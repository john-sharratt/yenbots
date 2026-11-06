// The one way a page calls the site (§6): JSON both ways, the `X-Yen-App`
// header the server requires of a browser's write, `If-Match` with the
// version a change was made against, every string sent in NFC (§8.7), and
// a refusal thrown as an `ApiError` carrying the server's `{error, detail,
// field}` — the shape the forms show beside a field. A call that fails in a
// way no page expects is told to `failed`, which the framework reports to
// the app's log (§18.1).

/** What the server answered instead of doing what was asked. */
export class ApiError extends Error {
  constructor(status, body) {
    super(body.detail || body.error || `the site answered ${status}`);
    this.name = "ApiError";
    this.status = status;
    this.code = body.error;
    this.detail = body.detail;
    this.field = body.field;
    this.reason = body.reason;
    this.version = body.version;
  }
}

// The answers that mean the site itself was not reached: the gateway
// standing in while the site restarts, or is too busy.
const UNREACHED = new Set([502, 503, 504]);

/**
 * The site could not be reached: no connection, the gateway answering for
 * it, or something else answering — a captive portal's page — in its place.
 */
export class Offline extends Error {
  constructor(cause) {
    super("there is no connection to the site");
    this.name = "Offline";
    this.cause = cause;
  }
}

// The refusals a page meets in its ordinary work — a form's mistake, a
// record someone else changed, a page not yours — which are not faults.
const EXPECTED = new Set(["invalid", "stale", "not_found", "no_app", "unauthorized", "not_active", "forbidden"]);

/** `value` with every string in it, keys included, in NFC. */
export function nfc(value) {
  if (typeof value === "string") return value.normalize("NFC");
  if (Array.isArray(value)) return value.map(nfc);
  if (value && typeof value === "object") {
    const out = {};
    for (const [k, v] of Object.entries(value)) out[k.normalize("NFC")] = nfc(v);
    return out;
  }
  return value;
}

/** `params` as a query string, leaving out what is absent: `?where=…&limit=50`. */
export function qs(params) {
  const parts = [];
  for (const [k, v] of Object.entries(params)) {
    if (v === undefined || v === null || v === "") continue;
    parts.push(`${encodeURIComponent(k)}=${encodeURIComponent(String(v))}`);
  }
  return parts.length ? `?${parts.join("&")}` : "";
}

export class Api {
  /**
   * @param app the app the page is, sent as `X-Yen-App`
   * @param fetch the browser's `fetch`
   * @param failed told `(method, path, error)` of each unexpected refusal
   */
  constructor(app, fetch, failed = () => {}) {
    this.app = app;
    this.fetch = fetch;
    this.failed = failed;
  }

  /** `method path` under `/api/v1`, with `body` as JSON; answers the JSON the site sent. */
  async call(method, path, body, { version } = {}) {
    const headers = { "X-Yen-App": this.app, Accept: "application/json" };
    if (body !== undefined) headers["Content-Type"] = "application/json";
    if (version !== undefined && version !== null) headers["If-Match"] = `"${version}"`;
    const init = { method, headers, credentials: "same-origin" };
    if (body !== undefined) init.body = JSON.stringify(nfc(body));
    let res;
    try {
      res = await this.fetch(`/api/v1${path}`, init);
    } catch (e) {
      throw new Offline(e);
    }
    if (UNREACHED.has(res.status)) throw new Offline(new Error(`the gateway answered ${res.status}`));
    const text = await res.text();
    let json = null;
    let parsed = true;
    try {
      json = text ? JSON.parse(text) : null;
    } catch {
      parsed = false;
    }
    if (res.ok && !(parsed && json !== null)) throw new Offline(new Error("the answer was not the site's"));
    if (res.ok) return json;
    const err = new ApiError(res.status, json ?? { error: `http_${res.status}`, detail: text.slice(0, 200) });
    if (!EXPECTED.has(err.code)) this.failed(method, path, err);
    throw err;
  }
}
