// An ER matter in a list: its title, who it is about, its kind, when it
// was opened and its status. It is given the words to show.
import { YenElement, html } from "/_/yen.js";

export class MatterRow extends YenElement {
  static properties = { matter: { attribute: false }, who: { type: String }, opened: { type: String }, status: { type: String } };

  render() {
    const m = this.matter;
    const about = [this.who, m.kind, this.opened && `opened ${this.opened}`].filter(Boolean).join(" · ");
    const tone = m.status === "closed" ? "neutral" : m.status === "on-hold" ? "warning" : "accent";
    return html`<yen-list-item href="m/${m.id}" headline=${m.title + (m.pending ? " (not sent yet)" : "")} supporting=${about}>
      <yen-icon slot="start" name="alert"></yen-icon>
      <yen-tag slot="end" tone=${tone}>${this.status}</yen-tag>
    </yen-list-item>`;
  }
}
customElements.define("matter-row", MatterRow);
