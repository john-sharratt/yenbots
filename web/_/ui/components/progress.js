// `<yen-progress value="0.4" label="Uploading">` (§8.5): how far something
// has gone, from 0 to 1 — or, with no `value`, that it is going. `kind` is
// bar (the default) or spinner. The `label` is what a screen reader says.

import { css, html, nothing } from "../../vendor/lit-all.min.js";
import { YenElement } from "../../framework/element.js";
import { base, spinner } from "../styles.js";

export class YenProgress extends YenElement {
  static properties = {
    value: { type: Number },
    label: { type: String },
    kind: { type: String, reflect: true },
  };

  static styles = [
    base,
    spinner,
    css`
      :host {
        display: block;
      }
      :host([kind="spinner"]) {
        display: inline-flex;
        color: var(--yen-accent);
      }
      :host([kind="spinner"]) .spinner {
        width: 1.75rem;
        height: 1.75rem;
        border-width: 3px;
      }
      .track {
        position: relative;
        height: 0.5rem;
        border-radius: var(--yen-radius-pill);
        background: var(--yen-surface-3);
        overflow: hidden;
      }
      .fill {
        height: 100%;
        border-radius: inherit;
        background: var(--yen-accent);
        transform-origin: left;
        transition: transform var(--yen-duration-3) var(--yen-ease-out);
      }
      .going {
        position: absolute;
        inset: 0;
        width: 40%;
        border-radius: inherit;
        background: var(--yen-accent);
        animation: go 1.2s var(--yen-ease-in-out) infinite;
      }
      @keyframes go {
        from {
          transform: translateX(-100%);
        }
        to {
          transform: translateX(250%);
        }
      }
      @media (prefers-reduced-motion: reduce) {
        .going {
          animation: none;
          width: 100%;
          opacity: 0.4;
        }
      }
    `,
  ];

  constructor() {
    super();
    this.kind = "bar";
  }

  updated() {
    // The fill's width is a transform set as a property, never inline markup.
    const fill = this.renderRoot.querySelector(".fill");
    if (fill) fill.style.transform = `scaleX(${Math.min(1, Math.max(0, this.value))})`;
  }

  render() {
    const known = typeof this.value === "number" && !Number.isNaN(this.value);
    const now = known ? Math.round(Math.min(1, Math.max(0, this.value)) * 100) : null;
    if (this.kind === "spinner") {
      return html`<span role="progressbar" aria-label=${this.label ?? "Working"} aria-valuenow=${now ?? nothing}
        ><span class="spinner"></span
      ></span>`;
    }
    return html`<div
      class="track"
      role="progressbar"
      aria-label=${this.label ?? "Working"}
      aria-valuemin="0"
      aria-valuemax="100"
      aria-valuenow=${now ?? nothing}
    >
      ${known ? html`<div class="fill"></div>` : html`<div class="going"></div>`}
    </div>`;
  }
}

customElements.define("yen-progress", YenProgress);
