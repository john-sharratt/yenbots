// The page's one app, made from the browser: the app its address names,
// its open counted (§20.3), its errors reported (§18.1), its writes kept in IndexedDB, its
// collections kept current by the change feed, `<yen-app>` and `<yen-bell>`
// defined, the service worker registered (§11.1), and the page reloaded when
// the site serves a new commit (§5.3).

import { signal } from "../vendor/signals-core.mjs";
import { Api } from "./api.js";
import { defineBell } from "./bell.js";
import { listenForChanges, listenForReload } from "./feed.js";
import { keyOf, openKeep } from "./idb.js";
import { browserDevice } from "./notify.js";
import { describe, Reporter } from "./report.js";
import { appIdOf } from "./router.js";
import { Sdk } from "./sdk.js";
import { defineShell } from "./shell.js";

/** How often writes left waiting are tried again, besides when the connection returns. */
const RETRY_MS = 30_000;

/** The app this page is, and how to move between its pages. */
export function boot() {
  const id = appIdOf(location.pathname) ?? "_site";
  const base = `/a/${id}/`;

  const reporter = new Reporter(id, (report) =>
    fetch("/api/v1/errors", {
      method: "POST",
      keepalive: true,
      headers: { "Content-Type": "application/json", "X-Yen-App": id },
      body: JSON.stringify(report),
    }).catch(() => {
      // A report that cannot be sent has nowhere else to go.
    }),
  );
  const report = (message, where) => reporter.error(message, where);

  // An uncaught error from the app's own files is shown on the page as well.
  const fault = signal(null);
  const ours = (where) => typeof where === "string" && where.includes(base);
  addEventListener("error", (e) => {
    const d = e.error ? describe(e.error) : { message: e.message, stack: `${e.filename}:${e.lineno}:${e.colno}` };
    report(d.message, { stack: d.stack, page: location.href });
    if (ours(e.filename)) fault.value = d.message;
  });
  addEventListener("unhandledrejection", (e) => {
    const d = describe(e.reason);
    report(d.message, { stack: d.stack, page: location.href });
    if (ours(d.stack)) fault.value = d.message;
  });
  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "hidden") reporter.flush();
  });

  const api = new Api(id, (url, init) => fetch(url, init), (method, path, err) =>
    report(`${method} ${path} answered ${err.status} ${err.code ?? ""}: ${err.detail ?? ""}`, { page: location.href }),
  );
  const keep = openKeep(`yenbot-${id}`);
  const sdk = new Sdk(id, {
    api,
    storage: keyOf(keep, "queue"),
    cache: keep,
    compare: new Intl.Collator("en-AU").compare,
    device: browserDevice(),
  });

  // The service worker shows the site's pushes (§11.1); a browser that
  // offers to put the site on the home screen has its offer kept for the
  // bell.
  navigator.serviceWorker?.register("/sw.js").catch((e) => report(`the service worker did not start: ${describe(e).message}`, { page: location.href }));
  const installable = signal(null);
  addEventListener("beforeinstallprompt", (e) => {
    e.preventDefault();
    installable.value = e;
  });
  defineBell(sdk, { app: id, installable });

  // Once the app is open: the open counted for the admins' usage (§20.3),
  // its feed, and its waiting writes tried again.
  let listening = false;
  const listen = () => {
    if (listening) return;
    listening = true;
    api.call("POST", `/opens/${id}`).catch(() => {
      // An open not counted is a number a little low, nothing more.
    });
    listenForChanges(`/api/v1/events/${id}`, EventSource, (event) => sdk.heard(event));
    addEventListener("online", () => sdk.queue.flush());
    setInterval(() => {
      if (sdk.queue.writes.peek().length > 0) sdk.queue.flush();
    }, RETRY_MS);
  };
  /** Start the app — again, after it failed to. */
  const open = () =>
    sdk
      .start()
      .then((opened) => {
        if (opened) listen();
      })
      .catch((e) => {
        const d = describe(e);
        report(d.message, { stack: d.stack, page: location.href });
        fault.value = d.message;
      });

  const shell = id === "_site" ? null : defineShell(sdk, { base, fault, report, restart: open });
  listenForReload(EventSource, document, () => location.reload());
  // The site's own pages are no app: they use the bell, not a collection.
  if (id !== "_site") open();

  return { sdk, go: shell ? shell.go : (to) => location.assign(to) };
}
