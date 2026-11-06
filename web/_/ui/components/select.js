// `<yen-select label="Kind" .options=${["soup", "salad"]} .value=${…}>`
// (§8.5): one choice of a few. An option is a value, or `{value, label}`
// when the words differ from what is stored; `placeholder` is shown while
// nothing is chosen.

import { css, html, live, nothing } from "../../vendor/lit-all.min.js";
import { YenField } from "../field-element.js";
import { base, control, field } from "../styles.js";
import "./icon.js";

/** An option as `{value, label}`. */
const optionOf = (o) => (o && typeof o === "object" ? { value: String(o.value), label: o.label ?? String(o.value) } : { value: String(o), label: String(o) });

export class YenSelect extends YenField {
  static properties = {
    ...YenField.properties,
    options: { attribute: false },
    placeholder: { type: String },
  };

  static styles = [
    base,
    control,
    field,
    css`
      :host {
        display: block;
      }
    `,
  ];

  constructor() {
    super();
    this.options = [];
    this.value = "";
  }

  render() {
    const current = this.formValue();
    const options = (this.options ?? []).map(optionOf);
    return html`<div class="field">
      ${this.labelPart()}
      <select
        class="control"
        id=${this.ids.control}
        .value=${live(current)}
        ?required=${this.required}
        ?disabled=${this.disabled}
        aria-invalid=${this.error ? "true" : "false"}
        aria-describedby=${this.describedBy()}
        @change=${(e) => this.take(e.target.value, true)}
      >
        ${this.placeholder || current === ""
          ? html`<option value="" ?disabled=${this.required}>${this.placeholder ?? ""}</option>`
          : nothing}
        ${options.map((o) => html`<option value=${o.value} ?selected=${o.value === current}>${o.label}</option>`)}
      </select>
      ${this.notes()}
    </div>`;
  }
}

customElements.define("yen-select", YenSelect);
