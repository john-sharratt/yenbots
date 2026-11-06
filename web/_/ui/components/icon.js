// `<yen-icon name="plus">` (§8.5): one of the interface's icons
// (`web/_/ui/icons.js`), drawn from the sprite in the ink around it, at the
// size of the text around it. Decorative unless given a `label`, which a
// screen reader reads instead.

import { css, html, ifDefined } from "../../vendor/lit-all.min.js";
import { YenElement } from "../../framework/element.js";
import { SPRITE } from "../icons.js";

export class YenIcon extends YenElement {
  static properties = {
    name: { type: String },
    label: { type: String },
  };

  static styles = css`
    :host {
      display: inline-flex;
      flex: none;
      width: var(--yen-icon-size, 1.25em);
      height: var(--yen-icon-size, 1.25em);
      vertical-align: -0.25em;
      color: inherit;
    }
    :host([hidden]) {
      display: none;
    }
    svg {
      width: 100%;
      height: 100%;
    }
  `;

  render() {
    const label = this.label || undefined;
    return html`<svg
      role=${label ? "img" : "presentation"}
      aria-hidden=${label ? "false" : "true"}
      aria-label=${ifDefined(label)}
      focusable="false"
    >
      <use href=${`${SPRITE}#${this.name}`}></use>
    </svg>`;
  }
}

customElements.define("yen-icon", YenIcon);
