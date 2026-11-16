// Something coming up: what it is, who, and when. Late ones say so.
import { YenElement, html } from "/_/yen.js";

export class UpcomingRow extends YenElement {
  static properties = { item: { attribute: false }, when: { type: String } };

  render() {
    const u = this.item;
    return html`<yen-list-item href=${u.href} headline=${u.label} supporting=${`${u.who} · ${this.when}`}>
      <yen-icon slot="start" name=${u.late ? "alert" : "calendar"}></yen-icon>
      ${u.late ? html`<yen-tag slot="end" tone="danger">Late</yen-tag>` : ""}
    </yen-list-item>`;
  }
}
customElements.define("upcoming-row", UpcomingRow);
