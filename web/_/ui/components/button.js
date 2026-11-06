// `<yen-button>` (§8.5): the button, or a link that looks like one.
//
//   <yen-button variant="primary" icon="plus" @click=${…}>Add a recipe</yen-button>
//   <yen-button href="new">Add a recipe</yen-button>
//   <yen-button type="submit" ?loading=${saving}>Save</yen-button>
//
// `variant` is primary (the default), secondary, ghost or danger; `size`
// small for a tighter one; `wide` fills its row. `loading` shows a spinner
// and takes no presses; `disabled` takes none and says so. A `submit` or
// `reset` button submits or resets the form it is in.

import { css, html, ifDefined, LitElement, nothing } from "../../vendor/lit-all.min.js";
import { YenElement } from "../../framework/element.js";
import { base, button, spinner } from "../styles.js";
import "./icon.js";

export class YenButton extends YenElement {
  static shadowRootOptions = { ...LitElement.shadowRootOptions, delegatesFocus: true };

  static properties = {
    variant: { type: String, reflect: true },
    size: { type: String },
    icon: { type: String },
    href: { type: String },
    type: { type: String },
    disabled: { type: Boolean, reflect: true },
    loading: { type: Boolean, reflect: true },
    wide: { type: Boolean, reflect: true },
    label: { type: String },
  };

  static styles = [
    base,
    button,
    spinner,
    css`
      :host {
        display: inline-flex;
      }
      :host([wide]) {
        display: flex;
      }
    `,
  ];

  constructor() {
    super();
    this.variant = "primary";
    this.type = "button";
    this.disabled = false;
    this.loading = false;
    this.wide = false;
  }

  classes() {
    return ["btn", this.variant, this.size === "small" ? "small" : "", this.wide ? "wide" : ""].join(" ").trim();
  }

  inner() {
    const lead = this.loading
      ? html`<span class="spinner" aria-hidden="true"></span>`
      : this.icon
        ? html`<yen-icon name=${this.icon}></yen-icon>`
        : nothing;
    return html`${lead}<slot></slot>`;
  }

  render() {
    const label = this.label || undefined;
    if (this.href) {
      const off = this.disabled || this.loading;
      return html`<a
        class=${this.classes()}
        href=${ifDefined(off ? undefined : this.href)}
        aria-disabled=${off ? "true" : "false"}
        aria-busy=${this.loading ? "true" : "false"}
        aria-label=${ifDefined(label)}
        role=${ifDefined(off ? "link" : undefined)}
        >${this.inner()}</a
      >`;
    }
    return html`<button
      class=${this.classes()}
      type="button"
      ?disabled=${this.disabled}
      aria-busy=${this.loading ? "true" : "false"}
      aria-label=${ifDefined(label)}
      @click=${this.pressed}
    >
      ${this.inner()}
    </button>`;
  }

  pressed(e) {
    if (this.loading) {
      e.stopPropagation();
      return;
    }
    // A button inside a shadow root is no part of the form outside it.
    const form = this.closest("form");
    if (!form) return;
    if (this.type === "submit") form.requestSubmit();
    else if (this.type === "reset") form.reset();
  }
}

customElements.define("yen-button", YenButton);
