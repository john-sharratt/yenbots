// A new plan: what, which day — the one the planner showed, when it named
// one — and the time when there is one.
import { YenElement, css, go, html, toast } from "/_/yen.js";
import { plans, today } from "../stores/plans.js";

export class PlannerNew extends YenElement {
  static properties = { title: { state: true }, day: { state: true }, time: { state: true }, saving: { state: true } };

  static styles = css`
    form {
      display: grid;
      gap: var(--yen-space-4);
    }
  `;

  constructor() {
    super();
    this.title = "";
    this.day = null;
    this.time = "";
    this.saving = false;
  }

  /** The day chosen, or the one the address names, or today. */
  get chosen() {
    return this.day ?? this.query?.get("day") ?? today();
  }

  async save(e) {
    e.preventDefault();
    if (!this.title.trim()) return;
    this.saving = true;
    try {
      await plans.add({ title: this.title.trim(), day: this.chosen, ...(this.time ? { time: this.time } : {}) });
      go("./", { replace: true });
    } catch (err) {
      toast(err.detail ?? "Couldn't save the plan", { tone: "danger" });
    } finally {
      this.saving = false;
    }
  }

  render() {
    return html`<yen-page heading="A new plan" back="./">
      <form @submit=${(e) => this.save(e)}>
        <yen-text-field label="What" required .value=${this.title} @input=${(e) => (this.title = e.detail)}></yen-text-field>
        <yen-date-picker label="Day" type="date" .value=${this.chosen} @change=${(e) => (this.day = e.detail)}></yen-date-picker>
        <yen-date-picker label="Time" type="time" hint="Leave it empty for all day" .value=${this.time} @change=${(e) => (this.time = e.detail)}></yen-date-picker>
        <yen-button type="submit" variant="primary" ?loading=${this.saving}>Add it</yen-button>
      </form>
    </yen-page>`;
  }
}
customElements.define("planner-new", PlannerNew);
