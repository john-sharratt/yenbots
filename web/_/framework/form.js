// `<yen-form>` (§8.6): a form built from a collection's fields, adding a
// record or changing one, with what the site refused shown beside the field
// it names.
//
//   <yen-form .collection=${recipes} .record=${recipe}
//             .fields=${["title", "serves"]} @saved=${…}></yen-form>
//
// `fields` names which fields, in order; every field a form can build when
// absent. `saved` carries the record — or null for one kept on the device
// until the connection is back, which the form says.

import { css, html, nothing } from "../vendor/lit-all.min.js";
import "../ui/components/icon.js";
import { base, button, control, field, spinner } from "../ui/styles.js";
import { ApiError } from "./api.js";
import { YenElement } from "./element.js";
import { controlOf, FieldError, fromInput, labelOf, toInput, writeOf } from "./fields.js";

const pad = (n) => String(n).padStart(2, "0");

/** This device's own time zone, as a `datetime-local` control reads and writes it. */
const here = {
  toLocal: (ms) => {
    const d = new Date(ms);
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
  },
  fromLocal: (text) => new Date(text).getTime(),
};

export class YenForm extends YenElement {
  static properties = {
    collection: { attribute: false },
    record: { attribute: false },
    fields: { attribute: false },
    submit: { type: String },
    errors: { state: true },
    busy: { state: true },
    note: { state: true },
  };

  // The design system's controls (`web/_/ui/styles.js`), so a form built
  // here looks as one built from `<yen-text-field>`s does.
  static styles = [
    base,
    button,
    control,
    field,
    spinner,
    css`
      :host {
        display: block;
      }
      form {
        display: grid;
        gap: var(--yen-space-4);
      }
      .field.checkbox {
        grid-template-columns: auto 1fr;
        align-items: center;
        column-gap: var(--yen-space-3);
        min-height: var(--yen-tap);
      }
      .field.checkbox .label {
        order: 2;
        font-size: var(--yen-text-md);
        font-weight: var(--yen-weight-medium);
      }
      .field.checkbox .error {
        grid-column: 1 / -1;
        order: 3;
      }
      input[type="checkbox"] {
        width: 1.5rem;
        height: 1.5rem;
        margin: 0;
        accent-color: var(--yen-accent);
      }
      .note {
        margin: 0;
        padding: var(--yen-space-2) var(--yen-space-3);
        border-radius: var(--yen-radius-2);
        background: var(--yen-surface-2);
        font-size: var(--yen-text-sm);
      }
      .actions {
        display: flex;
        justify-content: flex-end;
      }
    `,
  ];

  constructor() {
    super();
    this.errors = {};
    this.busy = false;
    this.note = "";
    this.submit = "Save";
  }

  /** The fields shown, with their definitions. */
  shown() {
    const d = this.collection?.described;
    if (!d) return [];
    const names = this.fields ?? Object.keys(d.fields);
    return names.filter((n) => d.fields[n] && controlOf(d.fields[n])).map((n) => [n, d.fields[n]]);
  }

  render() {
    if (!this.collection?.described) return html`<p role="status">Loading…</p>`;
    return html`<form @submit=${this.send} novalidate>
      ${this.shown().map(([name, f]) => this.control(name, f))}
      ${this.errors[""] ? html`<p class="error" role="alert"><yen-icon name="alert"></yen-icon>${this.errors[""]}</p>` : nothing}
      ${this.note ? html`<p class="note" role="status">${this.note}</p>` : nothing}
      <div class="actions">
        <button class="btn primary" type="submit" ?disabled=${this.busy} aria-busy=${this.busy ? "true" : "false"}>
          ${this.busy ? html`<span class="spinner" aria-hidden="true"></span>` : nothing}${this.submit}
        </button>
      </div>
    </form>`;
  }

  control(name, f) {
    const kind = controlOf(f);
    const value = toInput(f, this.record?.[name], here);
    const id = `field-${name}`;
    const error = this.errors[name];
    const invalid = error ? "true" : "false";
    const described = error ? `${id}-error` : nothing;
    let input;
    if (kind === "textarea") {
      input = html`<textarea class="control" id=${id} name=${name} .value=${value} aria-invalid=${invalid} aria-describedby=${described}></textarea>`;
    } else if (kind === "checkbox") {
      input = html`<input id=${id} name=${name} type="checkbox" .checked=${value} aria-describedby=${described} />`;
    } else {
      const type = kind === "tags" ? "text" : kind;
      input = html`<input
        class="control"
        id=${id}
        name=${name}
        type=${type}
        .value=${value}
        ?required=${f.required}
        aria-invalid=${invalid}
        aria-describedby=${described}
      />`;
    }
    return html`<div class="field ${kind}">
      <label class="label" for=${id}>${labelOf(name)}${f.required ? html`<span class="required" aria-hidden="true"> *</span>` : nothing}</label>
      ${input}
      ${error
        ? html`<p class="error" id=${`${id}-error`} role="alert"><yen-icon name="alert"></yen-icon>${labelOf(name)} ${error}</p>`
        : nothing}
    </div>`;
  }

  send = async (e) => {
    e.preventDefault();
    const form = e.currentTarget;
    const values = {};
    const errors = {};
    for (const [name, f] of this.shown()) {
      const el = form.elements.namedItem(name);
      const raw = controlOf(f) === "checkbox" ? el.checked : el.value;
      // A browser that lets letters into a number box reports it empty.
      if (el.validity?.badInput) {
        errors[name] = controlOf(f) === "number" ? "must be a number" : "is not one the field takes";
        continue;
      }
      try {
        values[name] = fromInput(name, f, raw, here);
      } catch (err) {
        if (!(err instanceof FieldError)) throw err;
        errors[name] = err.message;
      }
    }
    this.errors = errors;
    this.note = "";
    if (Object.keys(errors).length > 0) return;
    const write = writeOf(this.collection.described.fields, values, this.record);
    if (this.record && Object.keys(write).length === 0) {
      this.emit("saved", this.record);
      return;
    }
    this.busy = true;
    try {
      const saved = this.record
        ? await this.collection.update(this.record.id, write)
        : await this.collection.add(write);
      if (saved === null) this.note = "Saved on this device. It will be sent when you're back online.";
      // A form that added a record is ready for the next.
      if (!this.record) form.reset();
      this.emit("saved", saved);
    } catch (err) {
      const field = err instanceof ApiError && err.code === "invalid" ? (err.field ?? "").replace(/\[.*$/, "") : "";
      if (this.shown().some(([name]) => name === field)) {
        this.errors = { [field]: err.reason };
      } else {
        this.errors = { "": err.message };
      }
    } finally {
      this.busy = false;
    }
  };
}

customElements.define("yen-form", YenForm);
