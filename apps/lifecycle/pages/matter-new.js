// Opening an ER matter for a person.
import { YenElement, css, go, html, toast } from "/_/yen.js";
import { people } from "../stores/people.js";
import { MATTER_KINDS, openMatter } from "../stores/matters.js";
import { today } from "../stores/days.js";

export class MatterNew extends YenElement {
  static properties = { title: { state: true }, kind: { state: true }, opened: { state: true }, summary: { state: true }, saving: { state: true } };

  static styles = css`
    form {
      display: grid;
      gap: var(--yen-space-4);
    }
  `;

  constructor() {
    super();
    this.title = "";
    this.kind = "Grievance";
    this.opened = today();
    this.summary = "";
    this.saving = false;
  }

  async save(e) {
    e.preventDefault();
    const title = this.title.trim();
    if (!title) return;
    this.saving = true;
    try {
      const summary = this.summary.trim();
      const made = await openMatter({ person: this.params.person, title, kind: this.kind, opened: this.opened || today(), ...(summary ? { summary } : {}) });
      go(made ? `m/${made.id}` : `p/${this.params.person}`, { replace: true });
    } catch (err) {
      toast(err.detail ?? "Couldn't save it", { tone: "danger" });
    } finally {
      this.saving = false;
    }
  }

  render() {
    const p = people.get(this.params.person);
    return html`<yen-page heading=${p ? `A new matter for ${p.name}` : "A new matter"} back="p/${this.params.person}">
      <form @submit=${(e) => this.save(e)}>
        <yen-text-field label="What it is about" required hint="A short title, such as Complaint about rostering" .value=${this.title} @input=${(e) => (this.title = e.detail)}></yen-text-field>
        <yen-select label="Kind" .options=${MATTER_KINDS} .value=${this.kind} @change=${(e) => (this.kind = e.detail)}></yen-select>
        <yen-date-picker type="date" label="Opened" .value=${this.opened} @change=${(e) => (this.opened = e.detail ?? "")}></yen-date-picker>
        <yen-text-area label="Summary" rows="6" hint="What was raised, by whom and when" .value=${this.summary} @input=${(e) => (this.summary = e.detail)}></yen-text-area>
        <yen-button type="submit" variant="primary" ?loading=${this.saving}>Open the matter</yen-button>
      </form>
    </yen-page>`;
  }
}
customElements.define("matter-new", MatterNew);
