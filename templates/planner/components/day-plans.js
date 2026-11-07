// One day and what is on it. It is given the day's heading and its plans.
import { YenElement, css, html } from "/_/yen.js";

export class DayPlans extends YenElement {
  static properties = { heading: { type: String }, plans: { attribute: false }, today: { type: Boolean } };

  static styles = css`
    h3 {
      margin: 0 0 var(--yen-space-2);
      font-size: var(--yen-text-md);
    }
    .today h3 {
      color: var(--yen-accent);
    }
    p {
      margin: 0;
      color: var(--yen-ink-2);
    }
  `;

  render() {
    return html`<section class=${this.today ? "today" : ""}>
      <h3>${this.heading}</h3>
      <yen-list
        divided
        .items=${this.plans ?? []}
        .item=${(p) => html`<yen-list-item href="p/${p.id}" headline=${p.title} supporting=${p.time ?? ""}></yen-list-item>`}
        empty="Nothing on"
        empty-icon="calendar"
      ></yen-list>
    </section>`;
  }
}
customElements.define("day-plans", DayPlans);
