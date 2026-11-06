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

import { css, html, nothing } from "../vendor/lit-core.min.js";
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

  static styles = css`
    form {
      display: grid;
      gap: var(--yen-space-3, 0.75rem);
    }
    .field {
      display: grid;
      gap: var(--yen-space-1, 0.25rem);
    }
    .field.checkbox {
      grid-template-columns: auto 1fr;
      align-items: center;
    }
    .field.checkbox label {
      order: 2;
    }
    .error {
      color: var(--yen-danger, currentColor);
      margin: 0;
    }
    .note {
      margin: 0;
    }
  `;

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
      ${this.errors[""] ? html`<p class="error" role="alert">${this.errors[""]}</p>` : nothing}
      ${this.note ? html`<p class="note" role="status">${this.note}</p>` : nothing}
      <button type="submit" ?disabled=${this.busy}>${this.submit}</button>
    </form>`;
  }

  control(name, f) {
    const kind = controlOf(f);
    const value = toInput(f, this.record?.[name], here);
    const id = `field-${name}`;
    const error = this.errors[name];
    const invalid = error ? "true" : "false";
    let input;
    if (kind === "textarea") {
      input = html`<textarea id=${id} name=${name} .value=${value} aria-invalid=${invalid}></textarea>`;
    } else if (kind === "checkbox") {
      input = html`<input id=${id} name=${name} type="checkbox" .checked=${value} />`;
    } else {
      const type = kind === "tags" ? "text" : kind;
      input = html`<input id=${id} name=${name} type=${type} .value=${value} ?required=${f.required} aria-invalid=${invalid} />`;
    }
    return html`<div class="field ${kind}">
      <label for=${id}>${labelOf(name)}${f.required ? html`<span aria-hidden="true"> *</span>` : nothing}</label>
      ${input}
      ${error ? html`<p class="error" role="alert">${labelOf(name)} ${error}</p>` : nothing}
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
