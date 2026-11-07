// A new entry: the day, what happened, and a photo if there is one.
import { YenElement, css, go, html, toast } from "/_/yen.js";
import { today, write } from "../stores/entries.js";

export class JournalNew extends YenElement {
  static properties = { day: { state: true }, text: { state: true }, photo: { state: true }, saving: { state: true } };

  static styles = css`
    form {
      display: grid;
      gap: var(--yen-space-4);
    }
  `;

  constructor() {
    super();
    this.day = today();
    this.text = "";
    this.photo = null;
    this.saving = false;
  }

  async save(e) {
    e.preventDefault();
    if (!this.text.trim()) return;
    this.saving = true;
    try {
      const made = await write({ day: this.day, text: this.text.trim(), photo: this.photo });
      go(made ? `e/${made.id}` : "./", { replace: true });
    } catch (err) {
      toast(err.detail ?? "Couldn't save the entry", { tone: "danger" });
    } finally {
      this.saving = false;
    }
  }

  render() {
    return html`<yen-page heading="A new entry" back="./">
      <form @submit=${(e) => this.save(e)}>
        <yen-date-picker label="Day" type="date" .value=${this.day} @change=${(e) => (this.day = e.detail)}></yen-date-picker>
        <yen-text-area label="What happened" rows="8" required .value=${this.text} @input=${(e) => (this.text = e.detail)}></yen-text-area>
        <yen-image-picker label="A photo" @change=${(e) => (this.photo = e.detail[0] ?? null)}></yen-image-picker>
        <yen-button type="submit" variant="primary" ?loading=${this.saving}>Save the entry</yen-button>
      </form>
    </yen-page>`;
  }
}
customElements.define("journal-new", JournalNew);
