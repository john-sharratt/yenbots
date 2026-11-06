// `<yen-card href="r/7">` (§8.5): a raised panel holding one thing — a
// recipe, a reminder. With `href` the whole card is a link and answers a
// pointer; a picture in its `media` slot fills the top, edge to edge.
//
//   <yen-card href="r/${r.id}">
//     <img slot="media" src=${r.photo} alt="">
//     <h3>${r.title}</h3>
//     <p>Serves ${r.serves}</p>
//   </yen-card>

import { css, html } from "../../vendor/lit-all.min.js";
import { YenElement } from "../../framework/element.js";
import { base } from "../styles.js";

export class YenCard extends YenElement {
  static properties = {
    href: { type: String },
    flat: { type: Boolean, reflect: true },
  };

  static styles = [
    base,
    css`
      :host {
        display: block;
      }
      .card {
        display: flex;
        flex-direction: column;
        height: 100%;
        overflow: hidden;
        background: var(--yen-surface);
        color: inherit;
        border: 1px solid var(--yen-line);
        border-radius: var(--yen-radius-3);
        box-shadow: var(--yen-shadow-1);
        text-decoration: none;
        transition:
          box-shadow var(--yen-duration-2) var(--yen-ease-out),
          transform var(--yen-duration-2) var(--yen-ease-out),
          border-color var(--yen-duration-2) var(--yen-ease-out);
      }
      :host([flat]) .card {
        box-shadow: none;
      }
      a.card:hover,
      :host([demo~="hover"]) .card {
        box-shadow: var(--yen-shadow-2);
        transform: translateY(-2px);
        border-color: var(--yen-line-strong);
      }
      a.card:active,
      :host([demo~="pressed"]) .card {
        transform: scale(0.99);
        box-shadow: var(--yen-shadow-1);
      }
      :host([demo~="focus"]) .card {
        outline: 2px solid var(--yen-focus);
        outline-offset: 2px;
      }
      .body {
        padding: var(--yen-space-4);
        display: grid;
        gap: var(--yen-space-1);
      }
      ::slotted([slot="media"]) {
        display: block;
        width: 100%;
        aspect-ratio: 4 / 3;
        object-fit: cover;
        background: var(--yen-surface-2);
      }
      ::slotted(h3),
      ::slotted(h4) {
        margin: 0;
      }
      ::slotted(p) {
        margin: 0;
        color: var(--yen-ink-2);
      }
    `,
  ];

  render() {
    const inner = html`<slot name="media"></slot>
      <div class="body"><slot></slot></div>`;
    return this.href ? html`<a class="card" href=${this.href}>${inner}</a>` : html`<div class="card">${inner}</div>`;
  }
}

customElements.define("yen-card", YenCard);
