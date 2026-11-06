// `<yen-grid size="medium">` (§8.5): its children in as many columns as the
// width holds — one on a narrow phone, more on a wider screen. `size` is how
// wide a column is at least: small (9rem), medium (14rem, the default) or
// large (20rem).

import { css, html } from "../../vendor/lit-all.min.js";
import { YenElement } from "../../framework/element.js";
import { base } from "../styles.js";

export class YenGrid extends YenElement {
  static properties = { size: { type: String, reflect: true } };

  static styles = [
    base,
    css`
      :host {
        --column: 14rem;
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(min(var(--column), 100%), 1fr));
        gap: var(--yen-space-4);
      }
      :host([size="small"]) {
        --column: 9rem;
        gap: var(--yen-space-3);
      }
      :host([size="large"]) {
        --column: 20rem;
      }
    `,
  ];

  constructor() {
    super();
    this.size = "medium";
  }

  render() {
    return html`<slot></slot>`;
  }
}

customElements.define("yen-grid", YenGrid);
