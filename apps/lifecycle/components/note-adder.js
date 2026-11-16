// Writing a file note: the day, its kind and what happened. It raises `add`
// with `{day, kind, text}`.
import { YenElement, css, html } from "/_/yen.js";

export class NoteAdder extends YenElement {
  static properties = {
    kinds: { attribute: false },
    today: { type: String },
    day: { state: true },
    kind: { state: true },
    text: { state: true },
  };

  static styles = css`
    form {
      display: grid;
      gap: var(--yen-space-3);
      margin-bottom: var(--yen-space-4);
    }
  `;

  constructor() {
    super();
    this.kinds = [];
    this.day = "";
    this.kind = "File note";
    this.text = "";
  }

  send(e) {
    e.preventDefault();
    const text = this.text.trim();
    if (!text) return;
    this.emit("add", { day: this.day || this.today, kind: this.kind, text });
    this.text = "";
  }

  render() {
    return html`<form @submit=${(e) => this.send(e)}>
      <yen-grid size="small">
        <yen-date-picker type="date" label="Day" .value=${this.day || this.today} @change=${(e) => (this.day = e.detail ?? "")}></yen-date-picker>
        <yen-select label="Kind" .options=${this.kinds} .value=${this.kind} @change=${(e) => (this.kind = e.detail)}></yen-select>
      </yen-grid>
      <yen-text-area label="What happened" rows="4" .value=${this.text} @input=${(e) => (this.text = e.detail)}></yen-text-area>
      <yen-button type="submit" icon="plus">Add the note</yen-button>
    </form>`;
  }
}
customElements.define("note-adder", NoteAdder);
