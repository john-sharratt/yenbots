// What is on: the week, day by day, or the month with a dot on each day
// that has something, and the chosen day below it.
import { YenElement, css, html, repeat } from "/_/yen.js";
import { marked, on, plans, shifted, spoken, today, weekOf } from "../stores/plans.js";
import "../components/day-plans.js";

export class PlannerHome extends YenElement {
  static properties = { view: { state: true }, day: { state: true } };

  static styles = css`
    .week {
      display: grid;
      gap: var(--yen-space-5);
    }
    .move {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin: var(--yen-space-4) 0;
    }
    yen-calendar {
      margin: var(--yen-space-4) auto;
    }
  `;

  constructor() {
    super();
    this.view = "week";
    this.day = today();
  }

  week() {
    const days = weekOf(this.day);
    const now = today();
    return html`<div class="move">
        <yen-icon-button icon="chevron-left" label="The week before" @click=${() => (this.day = shifted(this.day, -7))}></yen-icon-button>
        <strong>Week of ${spoken(days[0])}</strong>
        <yen-icon-button icon="chevron-right" label="The week after" @click=${() => (this.day = shifted(this.day, 7))}></yen-icon-button>
      </div>
      <div class="week">
        ${repeat(days, (d) => d, (d) => html`<day-plans heading=${spoken(d)} ?today=${d === now} .plans=${on(d)}></day-plans>`)}
      </div>`;
  }

  month() {
    return html`<yen-calendar .value=${this.day} .marks=${marked.value} @change=${(e) => (this.day = e.detail)}></yen-calendar>
      <day-plans heading=${spoken(this.day)} ?today=${this.day === today()} .plans=${on(this.day)}></day-plans>`;
  }

  render() {
    return html`<yen-page heading="Planner">
      <yen-tabs
        label="How to see it"
        .tabs=${[
          { id: "week", label: "Week", icon: "list" },
          { id: "month", label: "Month", icon: "calendar" },
        ]}
        selected=${this.view}
        @change=${(e) => (this.view = e.detail)}
      >
        <div slot="week">${this.week()}</div>
        <div slot="month">${this.month()}</div>
      </yen-tabs>
      ${plans.mayAdd ? html`<yen-fab icon="plus" href="new?day=${this.day}">Add a plan</yen-fab>` : ""}
    </yen-page>`;
  }
}
customElements.define("planner-home", PlannerHome);
