// `<yen-tabs>` (§8.5): views of one page, one shown at a time. Each tab's
// panel is a child in the slot its id names; the arrows, Home and End move
// between tabs, as the ARIA practices describe.
//
//   <yen-tabs .tabs=${[{id: "all", label: "All"}, {id: "mine", label: "Mine"}]}
//             selected="all" @change=${(e) => (this.tab = e.detail)}>
//     <div slot="all">…</div>
//     <div slot="mine">…</div>
//   </yen-tabs>
//
// It raises `change` with the chosen tab's id.

import { css, html, nothing } from "../../vendor/lit-all.min.js";
import { YenElement } from "../../framework/element.js";
import { moveTo } from "../roving.js";
import { base } from "../styles.js";
import "./icon.js";

export class YenTabs extends YenElement {
  static properties = {
    tabs: { attribute: false },
    selected: { type: String, reflect: true },
    label: { type: String },
  };

  static styles = [
    base,
    css`
      :host {
        display: block;
      }
      [role="tablist"] {
        display: flex;
        gap: var(--yen-space-1);
        padding: var(--yen-space-1);
        margin-bottom: var(--yen-space-4);
        background: var(--yen-surface-2);
        border-radius: var(--yen-radius-button);
        overflow-x: auto;
        scrollbar-width: none;
      }
      [role="tab"] {
        flex: 1 0 auto;
        display: inline-flex;
        align-items: center;
        justify-content: center;
        gap: var(--yen-space-2);
        min-height: calc(var(--yen-control-height) - 0.5rem);
        padding: 0 var(--yen-space-4);
        border: none;
        border-radius: var(--yen-radius-button);
        background: transparent;
        color: var(--yen-ink-2);
        font: inherit;
        font-weight: var(--yen-weight-semibold);
        cursor: pointer;
        transition:
          background-color var(--yen-duration-2) var(--yen-ease-out),
          color var(--yen-duration-2) var(--yen-ease-out),
          box-shadow var(--yen-duration-2) var(--yen-ease-out);
      }
      [role="tab"]:hover {
        color: var(--yen-ink);
      }
      [role="tab"][aria-selected="true"] {
        background: var(--yen-surface);
        color: var(--yen-ink);
        box-shadow: var(--yen-shadow-1);
      }
      [role="tabpanel"]:focus-visible {
        outline-offset: 4px;
      }
    `,
  ];

  constructor() {
    super();
    this.tabs = [];
  }

  choose(id, focus = false) {
    if (id !== this.selected) {
      this.selected = id;
      this.emit("change", id);
    }
    if (focus) this.updateComplete.then(() => this.renderRoot.querySelector(`#tab-${CSS.escape(id)}`)?.focus());
  }

  keyed(e, at) {
    const to = moveTo(at, e.key, this.tabs.length);
    if (to === null) return;
    e.preventDefault();
    this.choose(this.tabs[to].id, true);
  }

  render() {
    const tabs = this.tabs ?? [];
    const selected = this.selected ?? tabs[0]?.id;
    return html`<div role="tablist" aria-label=${this.label ?? nothing}>
        ${tabs.map(
          (t, i) => html`<button
            role="tab"
            id=${`tab-${t.id}`}
            aria-selected=${t.id === selected ? "true" : "false"}
            aria-controls="panel"
            tabindex=${t.id === selected ? "0" : "-1"}
            @click=${() => this.choose(t.id)}
            @keydown=${(e) => this.keyed(e, i)}
          >
            ${t.icon ? html`<yen-icon name=${t.icon}></yen-icon>` : nothing}${t.label}
          </button>`,
        )}
      </div>
      <div role="tabpanel" id="panel" tabindex="0" aria-labelledby=${`tab-${selected}`}>
        <slot name=${selected ?? ""}></slot>
      </div>`;
  }
}

customElements.define("yen-tabs", YenTabs);
