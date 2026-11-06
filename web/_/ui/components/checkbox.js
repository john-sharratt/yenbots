// `<yen-checkbox label="Vegetarian" ?checked=${…}>` (§8.5): a yes or no in a
// form, or one of several ticked in a list. `indeterminate` shows a group
// some of whose items are ticked. It raises `change` with whether it is
// ticked.

import { css, html, live } from "../../vendor/lit-all.min.js";
import { YenField } from "../field-element.js";
import { base, field } from "../styles.js";
import "./icon.js";

export class YenCheckbox extends YenField {
  static properties = {
    ...YenField.properties,
    checked: { type: Boolean, reflect: true },
    indeterminate: { type: Boolean },
  };

  static styles = [
    base,
    field,
    css`
      :host {
        display: block;
      }
      label.row {
        display: flex;
        align-items: center;
        gap: var(--yen-space-3);
        min-height: var(--yen-tap);
        cursor: pointer;
        font-size: var(--yen-text-md);
      }
      input {
        appearance: none;
        flex: none;
        display: grid;
        place-content: center;
        width: 1.5rem;
        height: 1.5rem;
        margin: 0;
        border: 2px solid var(--yen-line-strong);
        border-radius: var(--yen-radius-1);
        background: var(--yen-surface);
        cursor: pointer;
        transition:
          background-color var(--yen-duration-1) var(--yen-ease-out),
          border-color var(--yen-duration-1) var(--yen-ease-out);
      }
      input::before {
        content: "";
        width: 0.8rem;
        height: 0.45rem;
        border-left: 0.16rem solid var(--yen-on-accent);
        border-bottom: 0.16rem solid var(--yen-on-accent);
        transform: translateY(-0.1rem) rotate(-45deg) scale(0);
        transition: transform var(--yen-duration-2) var(--yen-ease-spring);
      }
      input:checked,
      input:indeterminate {
        background: var(--yen-accent);
        border-color: var(--yen-accent);
      }
      input:checked::before {
        transform: translateY(-0.1rem) rotate(-45deg) scale(1);
      }
      input:indeterminate::before {
        height: 0;
        border-left: none;
        transform: none;
      }
      input:hover:not(:disabled),
      :host([demo~="hover"]) input {
        border-color: var(--yen-ink-2);
      }
      :host([demo~="focus"]) input {
        outline: 2px solid var(--yen-focus);
        outline-offset: 2px;
      }
      input:active:not(:disabled),
      :host([demo~="pressed"]) input {
        transform: scale(0.92);
      }
      input[aria-invalid="true"] {
        border-color: var(--yen-danger);
      }
      input:disabled,
      input:disabled + span {
        opacity: 0.45;
        cursor: not-allowed;
      }
    `,
  ];

  constructor() {
    super();
    this.checked = false;
    this.indeterminate = false;
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

  render() {
    return html`<div class="field">
      <label class="row">
        <input
          type="checkbox"
          id=${this.ids.control}
          .checked=${live(this.checked)}
          .indeterminate=${this.indeterminate}
          ?required=${this.required}
          ?disabled=${this.disabled}
          aria-invalid=${this.error ? "true" : "false"}
          aria-describedby=${this.describedBy()}
          @change=${(e) => {
            this.checked = e.target.checked;
            this.indeterminate = false;
            this.emit("change", this.checked);
          }}
        />
        <span>${this.label}</span>
      </label>
      ${this.notes()}
    </div>`;
  }
}

customElements.define("yen-checkbox", YenCheckbox);
