// `<yen-app>` (§8.6): what every app is shown inside — its title, the way
// back to the landing page and the account's picture; sign-in, waiting for
// approval and an app not yours, handled; the page the address names;
// writes waiting for the connection and those the site refused, to keep or
// drop; and a "something went wrong" panel that says what and offers a
// retry.
//
// It is also the router's half that touches the browser: a link inside the
// app moves to its page without loading, back and forward follow, and a
// page change runs as a view transition unless the device asks for less
// motion.

import { css, html, nothing } from "../vendor/lit-core.min.js";
import { signal } from "../vendor/signals-core.mjs";
import { YenElement } from "./element.js";
import { describe } from "./report.js";
import { match, within } from "./router.js";

/** What a held write did, in words. */
function what(write) {
  if (write.op === "create") return `A new ${write.coll} record`;
  if (write.op === "delete") return `Deleting a ${write.coll} record`;
  return `A change to a ${write.coll} record`;
}

/**
 * Define `<yen-app>` for `sdk`, the app at `base` (`/a/recipes/`): `fault`
 * is a signal the page's uncaught errors are put in, `report` posts one,
 * and `restart` starts the app again for "Try again".
 */
export function defineShell(sdk, { base, fault, report, restart }) {
  const path = signal(within(base, location.pathname) ?? "");
  const query = signal(location.search);
  const follow = () => {
    path.value = within(base, location.pathname) ?? "";
    query.value = location.search;
  };
  const online = signal(navigator.onLine);
  addEventListener("online", () => (online.value = true));
  addEventListener("offline", () => (online.value = false));
  addEventListener("popstate", follow);

  /** Show the page at `to`, an address inside the app. */
  const go = (to, { replace = false } = {}) => {
    const url = new URL(to, document.baseURI);
    const inside = url.origin === location.origin ? within(base, url.pathname) : null;
    if (inside === null) {
      location.assign(url.href);
      return;
    }
    const move = () => {
      history[replace ? "replaceState" : "pushState"](null, "", url.pathname + url.search + url.hash);
      follow();
      scrollTo(0, 0);
    };
    const still = matchMedia("(prefers-reduced-motion: reduce)").matches;
    if (document.startViewTransition && !still) document.startViewTransition(move);
    else move();
  };

  document.addEventListener("click", (e) => {
    if (e.defaultPrevented || e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
    const a = e.composedPath().find((n) => n instanceof HTMLAnchorElement && n.hasAttribute("href"));
    if (!a || a.target || a.hasAttribute("download")) return;
    const url = new URL(a.href);
    if (url.origin !== location.origin || within(base, url.pathname) === null) return;
    // With the base at the app's root, `#x` would read as the first page's:
    // it names a place on the page shown, which keeps showing.
    const written = a.getAttribute("href");
    if (written.startsWith("#")) {
      e.preventDefault();
      history.pushState(null, "", location.pathname + location.search + written);
      return;
    }
    if (url.hash && url.pathname === location.pathname && url.search === location.search) return;
    e.preventDefault();
    go(url.href);
  });

  class Shell extends YenElement {
    static properties = { broken: { state: true } };

    static styles = css`
      :host {
        display: block;
        min-height: 100vh;
      }
      header {
        display: flex;
        align-items: center;
        gap: var(--yen-space-3, 0.75rem);
        padding: var(--yen-space-3, 0.75rem) var(--yen-space-4, 1rem);
      }
      header h1 {
        flex: 1;
        margin: 0;
        font: inherit;
        font-weight: 600;
      }
      .home {
        color: inherit;
        text-decoration: none;
      }
      .avatar {
        width: 2rem;
        height: 2rem;
        border-radius: 50%;
      }
      main {
        padding: 0 var(--yen-space-4, 1rem) var(--yen-space-6, 2rem);
      }
      .strip,
      .panel {
        margin: 0 var(--yen-space-4, 1rem) var(--yen-space-3, 0.75rem);
        padding: var(--yen-space-3, 0.75rem);
        border-radius: var(--yen-radius-2, 0.5rem);
        background: var(--yen-surface-2, transparent);
      }
      .strip p,
      .panel p {
        margin: 0 0 var(--yen-space-2, 0.5rem);
      }
      .held {
        display: flex;
        flex-wrap: wrap;
        gap: var(--yen-space-2, 0.5rem);
        align-items: center;
      }
    `;

    constructor() {
      super();
      this.broken = null;
      this.shownFor = null;
      this.shown = null;
      // A page, or the shell itself, whose render threw.
      this.addEventListener("yen-error", (e) => {
        this.broken = describe(e.detail).message;
      });
    }

    updated() {
      document.title = sdk.title.value || "Yen Bots";
    }

    render() {
      const me = sdk.me.value;
      const picture = me?.account?.picture;
      return html`<header>
          <a class="home" href="/" aria-label="All the apps">←</a>
          <h1>${sdk.title.value}</h1>
          ${picture ? html`<img class="avatar" src=${picture} alt=${me.account.name} />` : nothing}
        </header>
        ${this.strip()}
        <main>${this.body(me)}</main>`;
    }

    strip() {
      const waiting = sdk.queue.writes.value.length;
      const held = sdk.queue.held.value;
      const lines = [];
      if (!online.value) lines.push(html`<p>You're offline. What's here is what was last loaded.</p>`);
      if (waiting > 0) {
        const n = waiting === 1 ? "1 change is" : `${waiting} changes are`;
        lines.push(html`<p role="status">${n} waiting to be sent.</p>`);
      }
      if (sdk.queue.fault.value) lines.push(html`<p>This browser can't keep changes made offline.</p>`);
      for (const item of held) {
        lines.push(html`<div class="held" role="alert">
          <span>${what(item.write)} couldn't be saved: ${item.why.detail}</span>
          ${sdk.queue.keepable(item)
            ? html`<button @click=${() => sdk.queue.resend(item).catch((e) => this.failed(e))}>Keep mine</button>`
            : nothing}
          <button @click=${() => sdk.queue.drop(item)}>Drop it</button>
        </div>`);
      }
      return lines.length ? html`<div class="strip">${lines}</div>` : nothing;
    }

    body(me) {
      const broken = this.broken ?? fault.value;
      if (broken) {
        return html`<div class="panel" role="alert">
          <p>Something went wrong: ${broken}</p>
          <button @click=${() => this.retry()}>Try again</button>
        </div>`;
      }
      if (!me) return html`<p role="status">Loading…</p>`;
      if (!me.signed_in) {
        const next = encodeURIComponent(location.href);
        return html`<div class="panel">
          <p>Sign in to use ${sdk.title.value || "this app"}.</p>
          <a href="/auth/login?next=${next}">Sign in with Google</a>
        </div>`;
      }
      if (me.account.status === "blocked") return html`<div class="panel"><p>This account can't use Yen Bots.</p></div>`;
      if (me.account.status !== "active") {
        return html`<div class="panel"><p>Yen has been told you're here, and will let you in soon.</p></div>`;
      }
      if (sdk.closed.value) {
        return html`<div class="panel">
          <p>This app isn't one you can open.</p>
          <a href="/">Back to all the apps</a>
        </div>`;
      }
      // The app's own modules may not have run yet.
      if (sdk.routes.value.length === 0) return html`<p role="status">Loading…</p>`;
      const m = match(sdk.routes.value, path.value);
      if (!m) {
        return html`<div class="panel">
          <p>There's no such page here.</p>
          <a href="./">Back to the start</a>
        </div>`;
      }
      return this.page(m);
    }

    /** The page `m` names, made once per address. */
    page(m) {
      const address = path.value + query.value;
      if (this.shownFor !== address) {
        try {
          const el = typeof m.page === "string" ? document.createElement(m.page) : new m.page();
          el.params = m.params;
          el.query = new URLSearchParams(query.value);
          this.shown = el;
          this.shownFor = address;
        } catch (e) {
          queueMicrotask(() => this.failed(e));
          return nothing;
        }
      }
      return this.shown;
    }

    failed(e) {
      const d = describe(e);
      report(d.message, { stack: d.stack, page: location.href });
      this.broken = d.message;
    }

    retry() {
      this.broken = null;
      fault.value = null;
      this.shownFor = null;
      restart();
      for (const c of sdk.collections.values()) c.reload();
    }
  }

  customElements.define("yen-app", Shell);
  return { go, path };
}
