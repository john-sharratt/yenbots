// What the answers come to: the days chosen, the average rating, and who is
// bringing what.
import { YenElement, css, html } from "/_/yen.js";
import { answers, bringing, byDay, rating } from "../stores/answers.js";

export class FormSummary extends YenElement {
  static styles = css`
    .rating {
      font-size: var(--yen-text-2xl);
      font-weight: var(--yen-weight-bold);
      margin: var(--yen-space-5) 0;
    }
    h2 {
      font-size: var(--yen-text-lg);
      margin: var(--yen-space-6) 0 var(--yen-space-3);
    }
  `;

  render() {
    const n = answers.value.length;
    return html`<yen-page heading="The answers" back="./">
      <p>${n === 1 ? "One person has answered." : `${n} people have answered.`}</p>
      <yen-chart kind="bar" label="Which day suits" .rows=${byDay.value} empty="No answers yet"></yen-chart>
      <p class="rating">${rating.value === null ? "No ratings yet" : `Excitement: ${rating.value} out of 5`}</p>
      <h2>Bringing</h2>
      <yen-list
        divided
        .items=${bringing.value}
        .item=${(a) => html`<yen-list-item headline=${a.bringing} supporting=${a.name}></yen-list-item>`}
        ?loading=${answers.loading.value}
        empty="Nobody has said yet"
        empty-icon="gift"
      ></yen-list>
    </yen-page>`;
  }
}
customElements.define("form-summary", FormSummary);
