// `<yen-text-field label="Title" name="title" .value=${…}>` (§8.5): a line
// of text — or an email, a number, a phone number, a web address, a search.
// `type` is the kind (text when absent); `placeholder`, `hint`, `error`,
// `required` and `disabled` as their names say; `icon` sits at its start.

import { css, html, ifDefined, live } from "../../vendor/lit-all.min.js";
import { YenField } from "../field-element.js";
import { base, control, field } from "../styles.js";
import "./icon.js";

export class YenTextField extends YenField {
  static properties = {
    ...YenField.properties,
    type: { type: String },
    placeholder: { type: String },
    icon: { type: String },
    inputmode: { type: String },
    autocomplete: { type: String },
  };

  static styles = [
    base,
    control,
    field,
    css`
      :host {
        display: block;
      }
      .wrap {
        position: relative;
      }
      .wrap yen-icon {
        position: absolute;
        left: var(--yen-space-3);
        top: 50%;
        transform: translateY(-50%);
        color: var(--yen-ink-2);
        pointer-events: none;
      }
      .wrap.with-icon .control {
        padding-left: calc(var(--yen-space-3) * 2 + 1.25rem);
      }
    `,
  ];

  constructor() {
    super();
    this.type = "text";
    this.value = "";
  }

  render() {
    const icon = this.icon || (this.type === "search" ? "search" : "");
    return html`<div class="field">
      ${this.labelPart()}
      <div class="wrap ${icon ? "with-icon" : ""}">
        ${icon ? html`<yen-icon name=${icon}></yen-icon>` : ""}
        <input
          class="control"
          id=${this.ids.control}
          type=${this.type}
          .value=${live(this.formValue())}
          placeholder=${ifDefined(this.placeholder)}
          inputmode=${ifDefined(this.inputmode)}
          autocomplete=${ifDefined(this.autocomplete)}
          ?required=${this.required}
          ?disabled=${this.disabled}
          aria-invalid=${this.error ? "true" : "false"}
          aria-describedby=${this.describedBy()}
          @input=${(e) => {
            e.stopPropagation();
            this.take(e.target.value, false);
          }}
          @change=${(e) => this.take(e.target.value, true)}
        />
      </div>
      ${this.notes()}
    </div>`;
  }
}

customElements.define("yen-text-field", YenTextField);
