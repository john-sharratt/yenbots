// The latest number, a chart of the last thirty, a way to log one, and the
// list of every one.
import { YenElement, css, html, toast } from "/_/yen.js";
import { entries, latest, money, newest, rows, short, today } from "../stores/entries.js";

export class TrackerHome extends YenElement {
  static properties = { value: { state: true }, day: { state: true }, note: { state: true } };

  static styles = css`
    .latest {
      font-size: var(--yen-text-3xl);
      font-weight: var(--yen-weight-bold);
      margin: 0 0 var(--yen-space-4);
    }
    form {
      display: grid;
      gap: var(--yen-space-3);
      margin: var(--yen-space-5) 0;
    }
  `;

  constructor() {
    super();
    this.value = "";
    this.day = today();
    this.note = "";
  }

  async log(e) {
    e.preventDefault();
    const value = Number(this.value);
    if (this.value === "" || Number.isNaN(value)) {
      toast("Write a number first", { tone: "danger" });
      return;
    }
    await entries.add({ day: this.day, value, ...(this.note.trim() ? { note: this.note.trim() } : {}) });
    this.value = "";
    this.note = "";
  }

  render() {
    return html`<yen-page heading="Savings">
      <p class="latest">${latest.value === null ? "Nothing logged yet" : money(latest.value)}</p>
      <yen-chart kind="line" label="The last thirty" .rows=${rows.value} .format=${money} empty="Log a number to start the chart"></yen-chart>
      ${entries.mayAdd
        ? html`<form @submit=${(e) => this.log(e)}>
            <yen-text-field label="How much now" type="number" inputmode="decimal" .value=${this.value} @input=${(e) => (this.value = e.detail)}></yen-text-field>
            <yen-date-picker label="Day" type="date" .value=${this.day} @change=${(e) => (this.day = e.detail)}></yen-date-picker>
            <yen-text-field label="A note" .value=${this.note} @input=${(e) => (this.note = e.detail)}></yen-text-field>
            <yen-button type="submit" variant="primary" icon="plus">Log it</yen-button>
          </form>`
        : ""}
      <yen-list
        divided
        .items=${newest.value}
        .item=${(e) => html`<yen-list-item headline=${money(e.value)} supporting=${e.note ? `${short(e.day)} — ${e.note}` : short(e.day)}></yen-list-item>`}
        ?loading=${entries.loading.value}
        empty="Nothing logged yet"
        empty-icon="star"
      ></yen-list>
    </yen-page>`;
  }
}
customElements.define("tracker-home", TrackerHome);
