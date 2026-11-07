// One plan: change it, or take it off.
import { YenElement, ask, go, html } from "/_/yen.js";
import { plans, spoken } from "../stores/plans.js";

export class PlannerPlan extends YenElement {
  async remove(p) {
    if (await ask({ heading: `Take ${p.title} off?`, confirm: "Take it off", tone: "danger" })) {
      await plans.remove(p.id);
      go("./", { replace: true });
    }
  }

  render() {
    const p = plans.get(this.params.id);
    if (!p) {
      return html`<yen-page heading="Plan" back="./">
        <p>${plans.loading.value ? "Loading…" : "There's no such plan."}</p>
      </yen-page>`;
    }
    return html`<yen-page heading=${p.title} back="./">
      <p>${spoken(p.day)}${p.time ? `, ${p.time}` : ""}</p>
      ${plans.mayEdit(p)
        ? html`<yen-form .collection=${plans} .record=${p} .fields=${["title", "day", "notes"]} submit="Save changes"></yen-form>
            <yen-button variant="danger" icon="trash" @click=${() => this.remove(p)}>Take it off</yen-button>`
        : p.notes
          ? html`<p>${p.notes}</p>`
          : ""}
    </yen-page>`;
  }
}
customElements.define("planner-plan", PlannerPlan);
