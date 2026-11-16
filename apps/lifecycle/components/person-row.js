// A person in a list: their name, role and team, and a word on where they
// are or what is close. It is given the words to show.
import { YenElement, html } from "/_/yen.js";

export class PersonRow extends YenElement {
  static properties = { person: { attribute: false }, stage: { type: String }, flag: { type: String } };

  render() {
    const p = this.person;
    const about = [p.role, p.team].filter(Boolean).join(" · ");
    return html`<yen-list-item href="p/${p.id}" headline=${p.name + (p.pending ? " (not sent yet)" : "")} supporting=${about}>
      <yen-avatar slot="start" name=${p.name} size="small"></yen-avatar>
      ${this.flag ? html`<yen-tag slot="end" tone="warning">${this.flag}</yen-tag>` : html`<yen-tag slot="end">${this.stage}</yen-tag>`}
    </yen-list-item>`;
  }
}
customElements.define("person-row", PersonRow);
