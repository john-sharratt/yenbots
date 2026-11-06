// `<yen-tag tone="accent">Vegetarian</yen-tag>` (§8.5): a short label on
// something — a kind, a state, a filter chosen. `tone` is neutral (the
// default), accent, success, warning or danger; `removable` adds a ✕ that
// raises `remove`.

import { css, html, nothing } from "../../vendor/lit-all.min.js";
import { YenElement } from "../../framework/element.js";
import { base } from "../styles.js";
import "./icon.js";

export class YenTag extends YenElement {
  static properties = {
    tone: { type: String, reflect: true },
    icon: { type: String },
    removable: { type: Boolean },
  };

  static styles = [
    base,
    css`
      :host {
        --fill: var(--yen-surface-2);
        --edge: var(--yen-line);
        display: inline-flex;
      }
      :host([tone="accent"]) {
        --fill: var(--yen-accent-soft);
        --edge: color-mix(in srgb, var(--yen-accent) 30%, transparent);
      }
      :host([tone="success"]) {
        --fill: var(--yen-success-soft);
        --edge: color-mix(in srgb, var(--yen-success) 30%, transparent);
      }
      :host([tone="warning"]) {
        --fill: var(--yen-warning-soft);
        --edge: color-mix(in srgb, var(--yen-warning) 30%, transparent);
      }
      :host([tone="danger"]) {
        --fill: var(--yen-danger-soft);
        --edge: color-mix(in srgb, var(--yen-danger) 30%, transparent);
      }
      .tag {
        display: inline-flex;
        align-items: center;
        gap: var(--yen-space-1);
        min-height: 1.75rem;
        padding: 0 var(--yen-space-3);
        border: 1px solid var(--edge);
        border-radius: var(--yen-radius-pill);
        background: var(--fill);
        color: var(--yen-ink);
        font-size: var(--yen-text-sm);
        font-weight: var(--yen-weight-medium);
        white-space: nowrap;
      }
      .tag.removable {
        padding-right: var(--yen-space-1);
      }
      button {
        display: grid;
        place-items: center;
        width: 1.5rem;
        height: 1.5rem;
        padding: 0;
        border: none;
        border-radius: 50%;
        background: transparent;
        color: var(--yen-ink-2);
        cursor: pointer;
      }
      button:hover {
        background: color-mix(in srgb, var(--yen-ink) 10%, transparent);
        color: var(--yen-ink);
      }
      yen-icon {
        --yen-icon-size: 1rem;
      }
    `,
  ];

  constructor() {
    super();
    this.tone = "neutral";
    this.removable = false;
  }

  render() {
    return html`<span class="tag ${this.removable ? "removable" : ""}">
      ${this.icon ? html`<yen-icon name=${this.icon}></yen-icon>` : nothing}
      <slot></slot>
      ${this.removable
        ? html`<button type="button" aria-label=${`Remove ${this.textContent.trim()}`} @click=${() => this.emit("remove")}>
            <yen-icon name="x"></yen-icon>
          </button>`
        : nothing}
    </span>`;
  }
}

customElements.define("yen-tag", YenTag);
