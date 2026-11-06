// `<yen-page>` (§8.5): a page of an app, inside the shell — its heading, a
// way back when it has one, its actions, and its content at a readable
// width, arriving with the page transition.
//
//   <yen-page heading="Recipes" back="./">
//     <yen-icon-button slot="actions" icon="search" label="Search" …></yen-icon-button>
//     …content…
//   </yen-page>
//
// `back` is the address the arrow goes to; with none there is no arrow.
// `wide` gives a page of grids or charts the whole width.

import { css, html, nothing } from "../../vendor/lit-all.min.js";
import { YenElement } from "../../framework/element.js";
import { base } from "../styles.js";
import "./icon-button.js";

export class YenPage extends YenElement {
  static properties = {
    heading: { type: String },
    back: { type: String },
    wide: { type: Boolean, reflect: true },
  };

  static styles = [
    base,
    css`
      :host {
        display: block;
        width: 100%;
        max-width: 44rem;
        margin: 0 auto;
        animation: arrive var(--yen-duration-3) var(--yen-ease-out) both;
      }
      :host([wide]) {
        max-width: 72rem;
      }
      header {
        display: flex;
        align-items: center;
        gap: var(--yen-space-2);
        min-height: var(--yen-tap);
        margin: var(--yen-space-2) 0 var(--yen-space-4);
      }
      h2,
      .spacer {
        flex: 1;
        margin: 0;
        font-family: var(--yen-font-heading);
        font-size: var(--yen-text-2xl);
        font-weight: var(--yen-weight-semibold);
        line-height: var(--yen-leading-tight);
        letter-spacing: -0.01em;
        text-wrap: balance;
      }
      .actions {
        display: flex;
        gap: var(--yen-space-1);
      }
      .content {
        display: grid;
        gap: var(--yen-space-4);
      }
      @keyframes arrive {
        from {
          opacity: 0;
          transform: translateY(0.5rem);
        }
      }
    `,
  ];

  render() {
    return html`<header>
        ${this.back ? html`<yen-icon-button icon="arrow-left" label="Back" href=${this.back}></yen-icon-button>` : nothing}
        ${this.heading ? html`<h2>${this.heading}</h2>` : html`<span class="spacer"></span>`}
        <div class="actions"><slot name="actions"></slot></div>
      </header>
      <div class="content"><slot></slot></div>`;
  }
}

customElements.define("yen-page", YenPage);
