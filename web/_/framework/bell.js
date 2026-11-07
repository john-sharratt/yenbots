// `<yen-bell>` (§11.2): the bell on the landing page and in every app's
// shell — how many notifications are unread, and, pressed, the inbox in a
// sheet. The sheet is also where this device's notifications are turned on:
// on an iPhone or iPad, by first adding the site to the home screen, which
// the sheet walks the person through; on a phone that offers it, the site
// can be put on the home screen from here too (§11.1). In an app, the
// app's own notifications can be stopped here; the inbox keeps them anyway.

import { css, html, nothing } from "../vendor/lit-all.min.js";
import "../ui/components/button.js";
import "../ui/components/icon.js";
import "../ui/components/icon-button.js";
import "../ui/components/list.js";
import "../ui/components/list-item.js";
import "../ui/components/sheet.js";
import "../ui/components/toggle.js";
import { base } from "../ui/styles.js";
import { YenElement } from "./element.js";

/** When a notification arrived, as Yen reads it. */
const when = (ms) => new Date(ms).toLocaleString("en-AU", { weekday: "short", day: "numeric", month: "short", hour: "numeric", minute: "2-digit" });

/**
 * Define `<yen-bell>` for `sdk`, the app `app` (`_site` on the landing
 * page); `installable` holds the browser's deferred install prompt, when it
 * offered one.
 */
export function defineBell(sdk, { app, installable }) {
  class Bell extends YenElement {
    static properties = { open: { state: true }, busy: { state: true } };

    static styles = [
      base,
      css`
        :host {
          position: relative;
          display: inline-flex;
        }
        .count {
          position: absolute;
          top: 0.2rem;
          right: 0.2rem;
          min-width: 1.15rem;
          height: 1.15rem;
          padding: 0 0.3rem;
          border-radius: var(--yen-radius-pill);
          background: var(--yen-danger);
          color: var(--yen-on-accent);
          font-size: var(--yen-text-xs);
          font-weight: var(--yen-weight-semibold);
          line-height: 1.15rem;
          text-align: center;
          pointer-events: none;
        }
        .device {
          display: grid;
          gap: var(--yen-space-2);
          margin-bottom: var(--yen-space-4);
          padding: var(--yen-space-3);
          border-radius: var(--yen-radius-2);
          background: var(--yen-surface-2);
          font-size: var(--yen-text-sm);
        }
        ol {
          margin: 0;
          padding-left: var(--yen-space-5);
        }
        li + li {
          margin-top: var(--yen-space-1);
        }
        .unread {
          width: 0.6rem;
          height: 0.6rem;
          border-radius: 50%;
          background: var(--yen-accent);
        }
        .empty {
          color: var(--yen-ink-2);
          font-size: var(--yen-text-sm);
        }
      `,
    ];

    constructor() {
      super();
      this.open = false;
      this.busy = false;
    }

    connectedCallback() {
      super.connectedCallback();
      const refresh = () => {
        sdk.inbox.load().catch(() => {});
        sdk.notify.check().catch(() => {});
      };
      refresh();
      navigator.serviceWorker?.addEventListener("message", (e) => {
        if (e.data?.type === "yen-notification") refresh();
      });
      document.addEventListener("visibilitychange", () => {
        if (document.visibilityState === "visible") refresh();
      });
    }

    render() {
      const unread = sdk.inbox.unread.value;
      const label = unread ? `Notifications, ${unread} unread` : "Notifications";
      return html`<yen-icon-button icon="bell" label=${label} @click=${() => (this.open = true)}></yen-icon-button>
        ${unread ? html`<span class="count" aria-hidden="true">${unread > 9 ? "9+" : unread}</span>` : nothing}
        <yen-sheet heading="Notifications" ?open=${this.open} @close=${() => (this.open = false)}>
          ${this.device()} ${this.muting()} ${this.items()}
          ${unread
            ? html`<yen-button slot="actions" variant="secondary" @click=${() => sdk.inbox.readAll().catch(() => {})}>Mark all read</yen-button>`
            : nothing}
        </yen-sheet>`;
    }

    async turnOn() {
      this.busy = true;
      try {
        await sdk.notify.enable();
      } finally {
        this.busy = false;
      }
    }

    device() {
      const state = sdk.notify.state.value;
      const prompt = installable.value;
      const add = prompt
        ? html`<yen-button variant="secondary" icon="download" @click=${() => prompt.prompt().then(() => (installable.value = null))}
            >Add Yen Bot to your home screen</yen-button
          >`
        : nothing;
      switch (state) {
        case "install":
          return html`<div class="device">
            <strong>To get notifications on this iPhone or iPad, put Yen Bot on your home screen first:</strong>
            <ol>
              <li>Tap the Share button <yen-icon name="share"></yen-icon> at the bottom of Safari.</li>
              <li>Choose <strong>Add to Home Screen</strong>, then <strong>Add</strong>.</li>
              <li>Open Yen Bot from your home screen, and tap the bell there to turn notifications on.</li>
            </ol>
          </div>`;
        case "off":
          return html`<div class="device">
            Get a notification on this device when an app has something for you.
            <yen-button icon="bell" ?loading=${this.busy} @click=${() => this.turnOn()}>Turn on notifications</yen-button>
            ${add}
          </div>`;
        case "blocked":
          return html`<div class="device">
            Notifications are blocked for this site in this browser's settings. They're still kept here. ${add}
          </div>`;
        case "unsupported":
          return html`<div class="device">This browser can't show notifications. They're still kept here. ${add}</div>`;
        case "on":
          return html`<div class="device">
            Notifications are on for this device.
            <yen-button variant="ghost" size="small" @click=${() => sdk.notify.disable().catch(() => {})}>Turn them off here</yen-button>
            ${add}
          </div>`;
        default:
          return nothing;
      }
    }

    muting() {
      if (app === "_site") return nothing;
      const muted = sdk.inbox.muted.value.includes(app);
      const name = sdk.title.value || "this app";
      return html`<yen-toggle
        label=${`Notifications from ${name}`}
        ?checked=${!muted}
        @change=${(e) => {
          const on = e.detail ?? e.target.checked;
          sdk.notify
            .mute(app, !on)
            .then(() => sdk.inbox.load())
            .catch(() => {});
        }}
      ></yen-toggle>`;
    }

    /** A notification tapped: marked read, then its page opened — not the other way, which would cut the marking off. */
    opened(e, item) {
      if (!item.open) {
        sdk.inbox.read(item.id).catch(() => {});
        return;
      }
      e.preventDefault();
      sdk.inbox
        .read(item.id)
        .catch(() => {})
        .finally(() => location.assign(item.open));
    }

    items() {
      const items = sdk.inbox.items.value;
      if (!items.length) return html`<p class="empty">Nothing yet. Notifications from the apps arrive here.</p>`;
      return html`<yen-list>
        ${items.map(
          (i) => html`<yen-list-item
            href=${i.open ?? nothing}
            headline=${i.title}
            supporting=${[i.body, when(i.created_ms)].filter(Boolean).join(" · ")}
            @click=${(e) => this.opened(e, i)}
          >
            ${i.read_ms === null ? html`<span slot="start" class="unread" aria-label="Unread"></span>` : nothing}
          </yen-list-item>`,
        )}
      </yen-list>`;
    }
  }

  customElements.define("yen-bell", Bell);
}
