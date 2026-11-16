// A person's details as a form. It is given the record (none for a new
// person) and the choices to offer; it raises `save` with the fields, a
// cleared one as null so it is removed.
import { YenElement, css, html, repeat } from "/_/yen.js";

const TEXT = [
  { name: "name", label: "Name", required: true },
  { name: "role", label: "Role" },
  { name: "team", label: "Team" },
  { name: "manager", label: "Manager" },
  { name: "location", label: "Location" },
];

const DATES = [
  { name: "start_date", label: "Start date" },
  { name: "probation_end", label: "Probation ends" },
  { name: "contract_end", label: "Contract ends" },
  { name: "last_day", label: "Last day" },
];

export class PersonEditor extends YenElement {
  static properties = {
    person: { attribute: false },
    employment: { attribute: false },
    stages: { attribute: false },
    submit: { type: String },
    saving: { type: Boolean },
    values: { state: true },
  };

  static styles = css`
    form {
      display: grid;
      gap: var(--yen-space-4);
    }
  `;

  constructor() {
    super();
    this.submit = "Save";
    this.values = this.fill(undefined);
  }

  fill(p) {
    const v = {};
    for (const f of [...TEXT, ...DATES]) v[f.name] = p?.[f.name] ?? "";
    v.notes = p?.notes ?? "";
    v.employment = p?.employment ?? "full-time";
    v.stage = p?.stage ?? "onboarding";
    return v;
  }

  willUpdate(changed) {
    if (changed.has("person")) this.values = this.fill(this.person);
  }

  set(name, value) {
    this.values = { ...this.values, [name]: value };
  }

  send(e) {
    e.preventDefault();
    if (!String(this.values.name ?? "").trim()) return;
    const isNew = !this.person;
    const out = {};
    for (const [k, raw] of Object.entries(this.values)) {
      if (k === "stage" && !this.stages) continue;
      const v = typeof raw === "string" ? raw.trim() : raw;
      if (v === "" || v == null) {
        if (!isNew) out[k] = null;
      } else {
        out[k] = v;
      }
    }
    this.emit("save", out);
  }

  render() {
    const v = this.values;
    return html`<form @submit=${(e) => this.send(e)}>
      ${repeat(
        TEXT,
        (f) => f.name,
        (f) => html`<yen-text-field label=${f.label} ?required=${f.required} .value=${v[f.name]} @input=${(e) => this.set(f.name, e.detail)}></yen-text-field>`,
      )}
      <yen-select label="Employment" .options=${this.employment ?? []} .value=${v.employment} @change=${(e) => this.set("employment", e.detail)}></yen-select>
      ${this.stages
        ? html`<yen-select
            label="Where they are"
            hint="Onboarding and Leaving start the standard checklist"
            .options=${this.stages}
            .value=${v.stage}
            @change=${(e) => this.set("stage", e.detail)}
          ></yen-select>`
        : ""}
      <yen-grid size="small">
        ${repeat(
          DATES,
          (f) => f.name,
          (f) => html`<yen-date-picker type="date" label=${f.label} .value=${v[f.name]} @change=${(e) => this.set(f.name, e.detail ?? "")}></yen-date-picker>`,
        )}
      </yen-grid>
      <yen-text-area label="About them" rows="4" .value=${v.notes} @input=${(e) => this.set("notes", e.detail)}></yen-text-area>
      <yen-button type="submit" variant="primary" ?loading=${this.saving}>${this.submit}</yen-button>
    </form>`;
  }
}
customElements.define("person-editor", PersonEditor);
