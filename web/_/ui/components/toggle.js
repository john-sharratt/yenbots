// `<yen-toggle label="Remind me" ?checked=${…}>` (§8.5): a switch, on or off,
// that takes effect at once — for a setting, not a form's question (that is
// a checkbox). It raises `change` with whether it is on.

import { css, html, nothing } from "../../vendor/lit-all.min.js";
import { YenField } from "../field-element.js";
import { base, field } from "../styles.js";
import "./icon.js";

export class YenToggle extends YenField {
  static properties = {
    ...YenField.properties,
    checked: { type: Boolean, reflect: true },
  };

  static styles = [
    base,
    field,
    css`
      :host {
        display: block;
      }
      .row {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: var(--yen-space-3);
        min-height: var(--yen-tap);
      }
      .row .label {
        font-size: var(--yen-text-md);
        font-weight: var(--yen-weight-medium);
        cursor: pointer;
      }
      button {
        position: relative;
        flex: none;
        width: 3.25rem;
        height: 2rem;
        padding: 0;
        border: none;
        border-radius: var(--yen-radius-pill);
        background: var(--yen-line-strong);
        cursor: pointer;
        transition: background-color var(--yen-duration-2) var(--yen-ease-out);
      }
      button::after {
        content: "";
        position: absolute;
        top: 0.25rem;
        left: 0.25rem;
        width: 1.5rem;
        height: 1.5rem;
        border-radius: 50%;
        background: var(--yen-surface);
        box-shadow: var(--yen-shadow-1);
        transition: transform var(--yen-duration-2) var(--yen-ease-spring);
      }
      button[aria-checked="true"] {
        background: var(--yen-accent);
      }
      button[aria-checked="true"]::after {
        transform: translateX(1.25rem);
      }
      button:hover:not(:disabled),
      :host([demo~="hover"]) button {
        filter: brightness(0.96);
      }
      :host([demo~="focus"]) button {
        outline: 2px solid var(--yen-focus);
        outline-offset: 2px;
      }
      button:active:not(:disabled)::after,
      :host([demo~="pressed"]) button::after {
        width: 1.75rem;
      }
      button:disabled {
        opacity: 0.45;
        cursor: not-allowed;
      }
    `,
  ];

  constructor() {
    super();
    this.checked = false;
  }

  formValue() {
    return this.checked ? "on" : null;
  }

  updated(changed) {
    if (changed.has("checked")) this.internals?.setFormValue(this.formValue());
  }

  connectedCallback() {
    super.connectedCallback();
    if (this.initialChecked === undefined) this.initialChecked = this.checked;
  }

  formResetCallback() {
    this.checked = this.initialChecked;
  }

  flip() {
    this.checked = !this.checked;
    this.emit("change", this.checked);
  }

  render() {
    return html`<div class="field">
      <div class="row">
        <label class="label" for=${this.ids.control}>${this.label}</label>
        <button
          id=${this.ids.control}
          type="button"
          role="switch"
          aria-checked=${this.checked ? "true" : "false"}
          aria-describedby=${this.describedBy()}
          ?disabled=${this.disabled}
          @click=${this.flip}
        ></button>
      </div>
      ${this.hint || this.error ? this.notes() : nothing}
    </div>`;
  }
}

customElements.define("yen-toggle", YenToggle);
