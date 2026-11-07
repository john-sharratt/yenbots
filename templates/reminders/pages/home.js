// What is coming up, what has gone by, and turning the pings on.
import { YenElement, app, css, html } from "/_/yen.js";
import { coming, past, reminders, when } from "../stores/reminders.js";

export class RemindersHome extends YenElement {
  static styles = css`
    h2 {
      font-size: var(--yen-text-lg);
      margin: var(--yen-space-6) 0 var(--yen-space-3);
    }
    .pings {
      margin-bottom: var(--yen-space-5);
    }
  `;

  row(r) {
    return html`<yen-list-item headline=${r.title} supporting=${when(r.due_ms)}>
      <yen-icon-button slot="end" icon="trash" label="Delete ${r.title}" @click=${() => reminders.remove(r.id)}></yen-icon-button>
    </yen-list-item>`;
  }

  render() {
    const off = app.notify.state.value === "off";
    return html`<yen-page heading="Reminders">
      ${off
        ? html`<div class="pings">
            <yen-button variant="secondary" icon="bell" @click=${() => app.notify.enable()}>Ping me on this phone</yen-button>
          </div>`
        : ""}
      <yen-list divided .items=${coming.value} .item=${(r) => this.row(r)} ?loading=${reminders.loading.value} empty="Nothing coming up" empty-icon="bell"></yen-list>
      ${past.value.length
        ? html`<h2>Gone by</h2>
            <yen-list divided .items=${past.value} .item=${(r) => this.row(r)}></yen-list>`
        : ""}
      ${reminders.mayAdd ? html`<yen-fab icon="plus" href="new">Add a reminder</yen-fab>` : ""}
    </yen-page>`;
  }
}
customElements.define("reminders-home", RemindersHome);
