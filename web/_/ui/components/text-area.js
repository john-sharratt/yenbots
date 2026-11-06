// `<yen-text-area label="Method" name="method" .value=${…}>` (§8.5): a
// paragraph of text, growing with what is written where the browser can.
// `rows` is how many lines it shows at first; `placeholder`, `hint`,
// `error`, `required` and `disabled` as for a text field.

import { css, html, ifDefined, live } from "../../vendor/lit-all.min.js";
import { YenField } from "../field-element.js";
import { base, control, field } from "../styles.js";
import "./icon.js";

export class YenTextArea extends YenField {
  static properties = {
    ...YenField.properties,
    rows: { type: Number },
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
      textarea.control {
        field-sizing: content;
        max-height: 60vh;
      }
    `,
  ];

  constructor() {
    super();
    this.rows = 4;
    this.value = "";
  }

  render() {
    return html`<div class="field">
      ${this.labelPart()}
      <textarea
        class="control"
        id=${this.ids.control}
        rows=${this.rows}
        .value=${live(this.formValue())}
        placeholder=${ifDefined(this.placeholder)}
        ?required=${this.required}
        ?disabled=${this.disabled}
        aria-invalid=${this.error ? "true" : "false"}
        aria-describedby=${this.describedBy()}
        @input=${(e) => {
          e.stopPropagation();
          this.take(e.target.value, false);
        }}
        @change=${(e) => this.take(e.target.value, true)}
      ></textarea>
      ${this.notes()}
    </div>`;
  }
}

customElements.define("yen-text-area", YenTextArea);
