// `<yen-toasts>` (§8.5): where the page's toasts show, at the foot of the
// screen and above any sheet or dialog. The shell holds one; a page shows a
// toast with `toast("Saved")` (`web/_/ui/toasts.js`). A toast's action is a
// button in it — "Undo" — and every toast can be dismissed.

import { css, html, nothing, repeat } from "../../vendor/lit-all.min.js";
import { YenElement } from "../../framework/element.js";
import { base } from "../styles.js";
import { toasts } from "../toasts.js";
import "./icon.js";

const ICON = { info: "info", success: "circle-check", danger: "alert" };

export class YenToasts extends YenElement {
  static styles = [
    base,
    css`
      :host {
        position: fixed;
        inset: auto 0 max(var(--yen-space-4), env(safe-area-inset-bottom)) 0;
        width: auto;
        height: auto;
        margin: 0 auto;
        padding: 0 var(--yen-space-4);
        border: none;
        background: transparent;
        overflow: visible;
        z-index: 50;
        pointer-events: none;
        display: grid;
        justify-items: center;
        gap: var(--yen-space-2);
      }
      .toast {
        pointer-events: auto;
        display: flex;
        align-items: center;
        gap: var(--yen-space-3);
        width: min(30rem, 100%);
        min-height: 3.25rem;
        padding: var(--yen-space-2) var(--yen-space-2) var(--yen-space-2) var(--yen-space-4);
        border-radius: var(--yen-radius-2);
        background: var(--yen-ink);
        color: var(--yen-bg);
        box-shadow: var(--yen-shadow-3);
        animation: rise var(--yen-duration-3) var(--yen-ease-spring);
      }
      .message {
        flex: 1;
      }
      .toast.danger yen-icon.lead {
        color: color-mix(in srgb, var(--yen-danger) 60%, var(--yen-bg));
      }
      button {
        min-height: 2.5rem;
        padding: 0 var(--yen-space-3);
        border: none;
        border-radius: var(--yen-radius-button);
        background: transparent;
        color: inherit;
        font: inherit;
        font-weight: var(--yen-weight-semibold);
        cursor: pointer;
      }
      button:hover {
        background: color-mix(in srgb, var(--yen-bg) 14%, transparent);
      }
      button.dismiss {
        display: grid;
        place-items: center;
        width: 2.5rem;
        padding: 0;
      }
      @keyframes rise {
        from {
          opacity: 0;
          transform: translateY(1rem) scale(0.96);
        }
      }
    `,
  ];

  connectedCallback() {
    super.connectedCallback();
    // In the top layer, so a toast shows over an open sheet or dialog.
    if (this.showPopover) {
      this.popover = "manual";
      this.showPopover();
    }
  }

  updated() {
    // The top layer stacks in the order things were shown: a toast shown
    // after a dialog opened is brought back above it.
    if (this.showPopover && this.isConnected && toasts.shown.peek().length > 0) {
      if (this.matches(":popover-open")) this.hidePopover();
      this.showPopover();
    }
  }

  render() {
    return html`${repeat(
      toasts.shown.value,
      (t) => t.id,
      (t) => html`<div class="toast ${t.tone}" role=${t.tone === "danger" ? "alert" : "status"}>
        <yen-icon class="lead" name=${ICON[t.tone] ?? "info"}></yen-icon>
        <span class="message">${t.message}</span>
        ${t.action
          ? html`<button
              type="button"
              @click=${() => {
                toasts.dismiss(t.id);
                t.action.run();
              }}
            >
              ${t.action.label}
            </button>`
          : nothing}
        <button class="dismiss" type="button" aria-label="Dismiss" @click=${() => toasts.dismiss(t.id)}>
          <yen-icon name="x"></yen-icon>
        </button>
      </div>`,
    )}`;
  }
}

customElements.define("yen-toasts", YenToasts);
