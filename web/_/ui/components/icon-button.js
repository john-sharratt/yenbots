// `<yen-icon-button icon="trash" label="Delete">` (§8.5): a button that is
// only an icon, as large as a fingertip. The `label` is required: it is what
// a screen reader says and what a pointer's tooltip shows. `variant` is
// ghost (the default), secondary, primary or danger; `href` makes it a link.

import { css, html, ifDefined, LitElement } from "../../vendor/lit-all.min.js";
import { YenElement } from "../../framework/element.js";
import { base, button } from "../styles.js";
import "./icon.js";

export class YenIconButton extends YenElement {
  static shadowRootOptions = { ...LitElement.shadowRootOptions, delegatesFocus: true };

  static properties = {
    icon: { type: String },
    label: { type: String },
    variant: { type: String, reflect: true },
    href: { type: String },
    disabled: { type: Boolean, reflect: true },
    pressed: { type: Boolean, reflect: true },
  };

  static styles = [
    base,
    button,
    css`
      :host {
        display: inline-flex;
      }
      .btn {
        width: var(--yen-tap);
        min-width: var(--yen-tap);
        height: var(--yen-tap);
        min-height: var(--yen-tap);
        padding: 0;
        border-radius: var(--yen-radius-pill);
        box-shadow: none;
      }
      .ghost {
        color: var(--yen-ink-2);
      }
      .ghost:hover:not([disabled]),
      :host([demo~="hover"]) .ghost {
        color: var(--yen-ink);
        background: var(--yen-surface-3);
      }
      .ghost[aria-pressed="true"] {
        color: var(--yen-accent);
        background: var(--yen-accent-soft);
      }
      yen-icon {
        --yen-icon-size: 1.35rem;
      }
    `,
  ];

  constructor() {
    super();
    this.variant = "ghost";
    this.disabled = false;
  }

  render() {
    const cls = `btn ${this.variant}`;
    const icon = html`<yen-icon name=${this.icon}></yen-icon>`;
    if (this.href && !this.disabled) {
      return html`<a class=${cls} href=${this.href} aria-label=${this.label} title=${this.label}>${icon}</a>`;
    }
    return html`<button
      class=${cls}
      type="button"
      ?disabled=${this.disabled}
      aria-label=${this.label}
      title=${this.label}
      aria-pressed=${ifDefined(this.hasAttribute("pressed") ? String(this.pressed) : undefined)}
    >
      ${icon}
    </button>`;
  }
}

customElements.define("yen-icon-button", YenIconButton);
