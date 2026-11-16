// The people by where they are in their time with us, what is coming up,
// and the ER matters still open.
import { YenElement, css, html, repeat } from "/_/yen.js";
import { STAGES, counts, inStage, nameOf, people, stageLabel } from "../stores/people.js";
import { flagOf, lateCount, upcoming } from "../stores/upcoming.js";
import { matters, openMatters, statusLabel } from "../stores/matters.js";
import { fromNow, showDay } from "../stores/days.js";
import { isDemo, seedDemo } from "../stores/demo.js";
import "../components/person-row.js";
import "../components/upcoming-row.js";
import "../components/matter-row.js";

export class LifecycleHome extends YenElement {
  static properties = { tab: { state: true }, stage: { state: true } };

  static styles = css`
    .stages {
      display: flex;
      flex-wrap: wrap;
      gap: var(--yen-space-2);
      margin-top: var(--yen-space-4);
      margin-bottom: var(--yen-space-4);
    }
    .panel {
      padding-top: var(--yen-space-4);
    }
  `;

  constructor() {
    super();
    this.tab = "people";
    this.stage = "onboarding";
  }

  connectedCallback() {
    super.connectedCallback();
    seedDemo();
  }

  render() {
    const late = lateCount.value;
    const tabs = [
      { id: "people", label: "People", icon: "users" },
      { id: "coming", label: late ? `Coming up (${late} late)` : "Coming up", icon: "calendar" },
      { id: "matters", label: `ER matters (${openMatters.value.length})`, icon: "alert" },
    ];
    return html`<yen-page heading="Employee Lifecycle">
      ${isDemo() ? "" : html`<yen-button slot="actions" variant="secondary" size="small" icon="sparkles" href="ask">Ask</yen-button>`}
      <yen-tabs .tabs=${tabs} selected=${this.tab} label="What to show" @change=${(e) => (this.tab = e.detail)}>
        <div slot="people">
          <div class="stages">
            ${repeat(
              STAGES,
              (s) => s.value,
              (s) => html`<yen-button size="small" variant=${this.stage === s.value ? "primary" : "secondary"} @click=${() => (this.stage = s.value)}
                >${s.label} (${counts.value[s.value] ?? 0})</yen-button
              >`,
            )}
          </div>
          <yen-list
            divided
            .items=${inStage(this.stage)}
            .item=${(p) => html`<person-row .person=${p} stage=${stageLabel(p.stage)} flag=${flagOf(p)}></person-row>`}
            ?loading=${people.loading.value}
            empty="Nobody here yet"
            empty-icon="users"
          ></yen-list>
        </div>
        <div slot="coming" class="panel">
          <yen-list
            divided
            .items=${upcoming.value}
            .item=${(u) => html`<upcoming-row .item=${u} when=${`${showDay(u.day)}, ${fromNow(u.day)}`}></upcoming-row>`}
            ?loading=${people.loading.value}
            empty="Nothing in the next few weeks"
            empty-icon="calendar"
          ></yen-list>
        </div>
        <div slot="matters" class="panel">
          <yen-list
            divided
            .items=${openMatters.value}
            .item=${(m) => html`<matter-row .matter=${m} who=${nameOf(m.person)} opened=${showDay(m.opened)} status=${statusLabel(m.status)}></matter-row>`}
            ?loading=${matters.loading.value}
            empty="No open matters"
            empty-icon="circle-check"
          ></yen-list>
        </div>
      </yen-tabs>
      ${people.mayAdd ? html`<yen-fab icon="plus" href="new">Add a person</yen-fab>` : ""}
    </yen-page>`;
  }
}
customElements.define("lifecycle-home", LifecycleHome);
