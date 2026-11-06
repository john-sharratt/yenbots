// `<yen-fab icon="plus" href="new">Add a recipe</yen-fab>` (§8.5): the
// floating action button — the one thing a page is for, at the thumb's
// corner, above everything but a sheet or dialog. Its words show beside the
// icon; with `compact` only the icon shows, and the words are its label.
// With no `href` it raises `click` as a button does.

import { css, html, LitElement } from "../../vendor/lit-all.min.js";
import { YenElement } from "../../framework/element.js";
import { base, button } from "../styles.js";
import "./icon.js";

export class YenFab extends YenElement {
  static shadowRootOptions = { ...LitElement.shadowRootOptions, delegatesFocus: true };

  static properties = {
    icon: { type: String },
    href: { type: String },
    compact: { type: Boolean, reflect: true },
    inline: { type: Boolean, reflect: true },
  };

  static styles = [
    base,
    button,
    css`
      :host {
        position: fixed;
        right: max(var(--yen-space-4), env(safe-area-inset-right));
        bottom: max(var(--yen-space-5), env(safe-area-inset-bottom));
        z-index: 20;
        animation: rise var(--yen-duration-3) var(--yen-ease-spring) both;
      }
      /* In the catalogue, where it sits in the flow. */
      :host([inline]) {
        position: static;
        animation: none;
      }
      .btn {
        min-height: 3.5rem;
        padding: 0 var(--yen-space-5) 0 var(--yen-space-4);
        border-radius: var(--yen-radius-pill);
        box-shadow: var(--yen-shadow-2);
        font-size: var(--yen-text-md);
      }
      .btn:hover,
      :host([demo~="hover"]) .btn {
        box-shadow: var(--yen-shadow-3);
      }
      :host([compact]) .btn {
        width: 3.5rem;
        padding: 0;
      }
      :host([compact]) .words {
        position: absolute;
        width: 1px;
        height: 1px;
        overflow: hidden;
        clip-path: inset(50%);
      }
      yen-icon {
        --yen-icon-size: 1.5rem;
      }
      @keyframes rise {
        from {
          opacity: 0;
          transform: translateY(1rem) scale(0.9);
        }
      }
    `,
  ];

  render() {
    const inner = html`<yen-icon name=${this.icon || "plus"}></yen-icon><span class="words"><slot></slot></span>`;
    if (this.href) return html`<a class="btn primary" href=${this.href}>${inner}</a>`;
    return html`<button class="btn primary" type="button">${inner}</button>`;
  }
}

customElements.define("yen-fab", YenFab);
