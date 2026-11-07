// `<yen-app>` (§8.6): what every app is shown inside — its title, the way
// back to the landing page, the bell and the account's picture; sign-in, waiting for
// approval and an app not yours, handled; the page the address names;
// writes waiting for the connection and those the site refused, to keep or
// drop; and a "something went wrong" panel that says what and offers a
// retry.
//
// It is also the router's half that touches the browser: a link inside the
// app moves to its page without loading, back and forward follow, and a
// page change runs as a view transition unless the device asks for less
// motion.

import { css, html, nothing } from "../vendor/lit-all.min.js";
import { signal } from "../vendor/signals-core.mjs";
import "../ui/components/avatar.js";
import "../ui/components/button.js";
import "../ui/components/empty.js";
import "../ui/components/icon.js";
import "../ui/components/icon-button.js";
import "../ui/components/progress.js";
import "../ui/components/toasts.js";
// `base` is the app's address in here; the shared styles take another name.
import { base as shared } from "../ui/styles.js";
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

    static styles = [
      shared,
      css`
        :host {
          display: block;
          min-height: 100vh;
          min-height: 100dvh;
        }
        header {
          position: sticky;
          top: 0;
          z-index: 10;
          display: flex;
          align-items: center;
          gap: var(--yen-space-2);
          padding: max(var(--yen-space-2), env(safe-area-inset-top)) var(--yen-space-3) var(--yen-space-2);
          background: color-mix(in srgb, var(--yen-bg) 86%, transparent);
          backdrop-filter: saturate(1.4) blur(12px);
          -webkit-backdrop-filter: saturate(1.4) blur(12px);
          border-bottom: 1px solid color-mix(in srgb, var(--yen-line) 60%, transparent);
        }
        header h1 {
          flex: 1;
          margin: 0;
          min-width: 0;
          overflow-x: clip;
          text-overflow: ellipsis;
          white-space: nowrap;
          font-family: var(--yen-font-heading);
          font-size: var(--yen-text-lg);
          font-weight: var(--yen-weight-semibold);
          line-height: var(--yen-leading-tight);
        }
        yen-avatar {
          margin-right: var(--yen-space-1);
        }
        main {
          padding: var(--yen-space-3) var(--yen-space-4) calc(var(--yen-space-8) + env(safe-area-inset-bottom));
        }
        .strip {
          display: grid;
          gap: var(--yen-space-2);
          max-width: 44rem;
          margin: var(--yen-space-3) auto 0;
          padding: 0 var(--yen-space-4);
        }
        .note {
          display: flex;
          flex-wrap: wrap;
          align-items: center;
          gap: var(--yen-space-2) var(--yen-space-3);
          padding: var(--yen-space-2) var(--yen-space-3);
          border-radius: var(--yen-radius-2);
          background: var(--yen-surface-2);
          color: var(--yen-ink);
          font-size: var(--yen-text-sm);
        }
        .note.held {
          background: var(--yen-warning-soft);
        }
        .note.fault {
          background: var(--yen-danger-soft);
        }
        .note .words {
          flex: 1 1 14rem;
        }
        .note yen-icon {
          color: var(--yen-ink-2);
        }
        .answers {
          display: flex;
          gap: var(--yen-space-2);
        }
        .waiting {
          display: grid;
          place-items: center;
          padding: var(--yen-space-8) 0;
        }
        .wrong {
          max-width: 32rem;
          margin: var(--yen-space-6) auto;
        }
      `,
    ];

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
      document.title = sdk.title.value || "Yen Bot";
    }

    render() {
      const me = sdk.me.value;
      const account = me?.signed_in ? me.account : null;
      return html`<header>
          <yen-icon-button icon="home" label="All the apps" href="/"></yen-icon-button>
          <h1>${sdk.title.value}</h1>
          ${account?.status === "active" ? html`<yen-bell></yen-bell>` : nothing}
          ${account ? html`<yen-avatar size="small" name=${account.name} src=${account.picture ?? nothing}></yen-avatar>` : nothing}
        </header>
        ${this.strip()}
        <main>${this.body(me)}</main>
        <yen-toasts></yen-toasts>`;
    }

    note(icon, words, { kind = "", role = "status", answers = nothing } = {}) {
      return html`<div class="note ${kind}" role=${role}>
        <yen-icon name=${icon}></yen-icon><span class="words">${words}</span>${answers}
      </div>`;
    }

    strip() {
      const waiting = sdk.queue.writes.value.length;
      const held = sdk.queue.held.value;
      const lines = [];
      if (!online.value) lines.push(this.note("offline", "You're offline. What's here is what was last loaded."));
      if (waiting > 0) {
        const n = waiting === 1 ? "1 change is" : `${waiting} changes are`;
        lines.push(this.note("clock", `${n} waiting to be sent.`));
      }
      if (sdk.queue.fault.value) {
        lines.push(this.note("alert", "This browser can't keep changes made offline.", { kind: "fault", role: "alert" }));
      }
      for (const item of held) {
        const answers = html`<span class="answers">
          ${sdk.queue.keepable(item)
            ? html`<yen-button size="small" variant="secondary" @click=${() => sdk.queue.resend(item).catch((e) => this.failed(e))}
                >Keep mine</yen-button
              >`
            : nothing}
          <yen-button size="small" variant="ghost" @click=${() => sdk.queue.drop(item)}>Drop it</yen-button>
        </span>`;
        lines.push(this.note("alert", `${what(item.write)} couldn't be saved: ${item.why.detail}`, { kind: "held", role: "alert", answers }));
      }
      return lines.length ? html`<div class="strip">${lines}</div>` : nothing;
    }

    loading() {
      return html`<div class="waiting"><yen-progress kind="spinner" label="Loading"></yen-progress></div>`;
    }

    body(me) {
      const broken = this.broken ?? fault.value;
      if (broken) {
        return html`<yen-empty class="wrong" icon="alert" heading="Something went wrong" role="alert">
          ${broken}
          <yen-button slot="action" icon="refresh" @click=${() => this.retry()}>Try again</yen-button>
        </yen-empty>`;
      }
      if (!me) return this.loading();
      if (!me.signed_in) {
        const next = encodeURIComponent(location.href);
        return html`<yen-empty icon="lock" heading=${`Sign in to use ${sdk.title.value || "this app"}`}>
          Yen Bot's apps open once you've signed in.
          <yen-button slot="action" href=${`/auth/login?next=${next}`}>Sign in with Google</yen-button>
        </yen-empty>`;
      }
      if (me.account.status === "blocked") {
        return html`<yen-empty icon="lock" heading="This account can't use Yen Bot"></yen-empty>`;
      }
      if (me.account.status !== "active") {
        return html`<yen-empty icon="clock" heading="Nearly there">Yen has been told you're here, and will let you in soon.</yen-empty>`;
      }
      if (sdk.closed.value) {
        return html`<yen-empty icon="lock" heading="This app isn't one you can open">
          <yen-button slot="action" variant="secondary" href="/">Back to all the apps</yen-button>
        </yen-empty>`;
      }
      // The app's own modules may not have run yet.
      if (sdk.routes.value.length === 0) return this.loading();
      const m = match(sdk.routes.value, path.value);
      if (!m) {
        return html`<yen-empty icon="search" heading="There's no such page here">
          <yen-button slot="action" variant="secondary" href="./">Back to the start</yen-button>
        </yen-empty>`;
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
