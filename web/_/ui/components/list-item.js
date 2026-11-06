// `<yen-list-item>` (§8.5): one row of a list — something at its start (an
// avatar, an icon, a checkbox), its words, and something at its end (a
// time, a count, an action). With `href` the row is a link, with an arrow.
//
//   <yen-list-item href="r/7" headline="Phở bò" supporting="Serves 4">
//     <yen-avatar slot="start" name="Phở bò"></yen-avatar>
//     <span slot="end">20 min</span>
//   </yen-list-item>

import { css, html, nothing } from "../../vendor/lit-all.min.js";
import { YenElement } from "../../framework/element.js";
import { base } from "../styles.js";
import "./icon.js";

export class YenListItem extends YenElement {
  static properties = {
    href: { type: String },
    headline: { type: String },
    supporting: { type: String },
  };

  static styles = [
    base,
    css`
      :host {
        display: block;
      }
      .row {
        display: flex;
        align-items: center;
        gap: var(--yen-space-3);
        min-height: calc(3.5rem * var(--yen-density));
        padding: var(--yen-space-2) var(--yen-space-4);
        color: inherit;
        text-decoration: none;
        background: var(--yen-surface);
        transition: background-color var(--yen-duration-1) var(--yen-ease-out);
      }
      a.row:hover,
      :host([demo~="hover"]) .row {
        background: var(--yen-surface-2);
      }
      a.row:active,
      :host([demo~="pressed"]) .row {
        background: var(--yen-surface-3);
      }
      :host([demo~="focus"]) .row {
        outline: 2px solid var(--yen-focus);
        outline-offset: -2px;
      }
      .words {
        flex: 1;
        min-width: 0;
        display: grid;
      }
      /* Cut short sideways only: a mark above a Vietnamese capital stands above the line box. */
      .headline {
        font-weight: var(--yen-weight-medium);
        overflow-x: clip;
        text-overflow: ellipsis;
        white-space: nowrap;
      }
      .supporting {
        font-size: var(--yen-text-sm);
        color: var(--yen-ink-2);
        overflow-x: clip;
        text-overflow: ellipsis;
        white-space: nowrap;
      }
      .end {
        display: flex;
        align-items: center;
        gap: var(--yen-space-2);
        color: var(--yen-ink-2);
        font-size: var(--yen-text-sm);
      }
      .chevron {
        color: var(--yen-ink-3);
      }
    `,
  ];

  render() {
    const inner = html`<slot name="start"></slot>
      <span class="words">
        <span class="headline">${this.headline}<slot></slot></span>
        ${this.supporting ? html`<span class="supporting">${this.supporting}</span>` : nothing}
        <slot name="supporting"></slot>
      </span>
      <span class="end">
        <slot name="end"></slot>
        ${this.href ? html`<yen-icon class="chevron" name="chevron-right"></yen-icon>` : nothing}
      </span>`;
    return this.href ? html`<a class="row" href=${this.href}>${inner}</a>` : html`<div class="row">${inner}</div>`;
  }
}

customElements.define("yen-list-item", YenListItem);
