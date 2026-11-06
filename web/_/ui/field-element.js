// What every form control of the design system shares (§8.5): a label, a
// hint, an error shown beside it and read with it, `required` and
// `disabled`, and a place in the form around it — each is a form-associated
// element, so a `<form>` submits its `name` and `value` and resets it as it
// would a native control.
//
// A control raises `input` as its value changes and `change` when it is
// settled, both carrying the value as `detail`.

import { html, LitElement, nothing } from "../vendor/lit-all.min.js";
import { YenElement } from "../framework/element.js";
import "./components/icon.js";

let made = 0;

export class YenField extends YenElement {
  static formAssociated = true;
  static shadowRootOptions = { ...LitElement.shadowRootOptions, delegatesFocus: true };

  static properties = {
    label: { type: String },
    name: { type: String, reflect: true },
    value: {},
    hint: { type: String },
    error: { type: String },
    required: { type: Boolean, reflect: true },
    disabled: { type: Boolean, reflect: true },
  };

  constructor() {
    super();
    this.internals = this.attachInternals?.() ?? null;
    this.uid = `yen-field-${++made}`;
    this.required = false;
    this.disabled = false;
    this.initial = undefined;
  }

  connectedCallback() {
    super.connectedCallback();
    if (this.initial === undefined) this.initial = this.value;
  }

  updated(changed) {
    if (changed.has("value")) this.internals?.setFormValue(this.formValue());
  }

  /** The value as the form submits it. */
  formValue() {
    return this.value === undefined || this.value === null ? "" : String(this.value);
  }

  /** The form was reset: the value the control was given first. */
  formResetCallback() {
    this.value = this.initial;
  }

  formDisabledCallback(disabled) {
    this.disabled = disabled;
  }

  /** Take `value` from the person: `input` while it changes, `change` once `settled`. */
  take(value, settled) {
    this.value = value;
    this.emit(settled ? "change" : "input", value);
  }

  get ids() {
    return { control: `${this.uid}-control`, hint: `${this.uid}-hint`, error: `${this.uid}-error` };
  }

  /** The ids the control is described by: its hint and its error. */
  describedBy() {
    return [this.hint ? this.ids.hint : "", this.error ? this.ids.error : ""].filter(Boolean).join(" ") || nothing;
  }

  labelPart() {
    if (!this.label) return nothing;
    return html`<label class="label" for=${this.ids.control}
      >${this.label}${this.required ? html`<span class="required" aria-hidden="true"> *</span>` : nothing}</label
    >`;
  }

  notes() {
    return html`${this.hint ? html`<p class="hint" id=${this.ids.hint}>${this.hint}</p>` : nothing}
    ${this.error
      ? html`<p class="error" id=${this.ids.error} role="alert"><yen-icon name="alert"></yen-icon>${this.error}</p>`
      : nothing}`;
  }
}
