// `<yen-skeleton>` (§8.5): the shape of what is still loading, shimmering
// where motion is welcome. `shape` is text (lines of words, `lines` of them),
// circle (an avatar) or block (a card or a row).

import { css, html } from "../../vendor/lit-all.min.js";
import { YenElement } from "../../framework/element.js";
import { base } from "../styles.js";

export class YenSkeleton extends YenElement {
  static properties = {
    shape: { type: String, reflect: true },
    lines: { type: Number },
  };

  static styles = [
    base,
    css`
      :host {
        display: block;
      }
      .bone {
        background: linear-gradient(
          100deg,
          var(--yen-surface-2) 30%,
          var(--yen-surface-3) 50%,
          var(--yen-surface-2) 70%
        );
        background-size: 300% 100%;
        animation: shimmer 1.4s ease-in-out infinite;
        border-radius: var(--yen-radius-1);
      }
      .line {
        height: 0.9em;
        margin: 0.45em 0;
      }
      .line:last-child:not(:first-child) {
        width: 62%;
      }
      :host([shape="circle"]) .bone {
        width: 2.75rem;
        height: 2.75rem;
        border-radius: 50%;
      }
      :host([shape="block"]) .bone {
        height: 4.5rem;
        border-radius: var(--yen-radius-3);
      }
      @keyframes shimmer {
        from {
          background-position: 100% 0;
        }
        to {
          background-position: 0 0;
        }
      }
      @media (prefers-reduced-motion: reduce) {
        .bone {
          animation: none;
        }
      }
    `,
  ];

  constructor() {
    super();
    this.shape = "text";
    this.lines = 3;
  }

  render() {
    if (this.shape !== "text") return html`<div class="bone" aria-hidden="true"></div>`;
    const n = Math.max(1, this.lines | 0);
    return html`${Array.from({ length: n }, () => html`<div class="bone line" aria-hidden="true"></div>`)}`;
  }
}

customElements.define("yen-skeleton", YenSkeleton);
