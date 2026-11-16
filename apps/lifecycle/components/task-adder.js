// Adding one item to a checklist: its words and, if it has one, a due day.
// It raises `add` with `{title, due}`.
import { YenElement, css, html } from "/_/yen.js";

export class TaskAdder extends YenElement {
  static properties = { title: { state: true }, due: { state: true } };

  static styles = css`
    form {
      display: flex;
      flex-wrap: wrap;
      align-items: end;
      gap: var(--yen-space-3);
      margin-top: var(--yen-space-3);
    }
    yen-text-field {
      flex: 1;
    }
  `;

  constructor() {
    super();
    this.title = "";
    this.due = "";
  }

  send(e) {
    e.preventDefault();
    const title = this.title.trim();
    if (!title) return;
    this.emit("add", { title, due: this.due || null });
    this.title = "";
    this.due = "";
  }

  render() {
    return html`<form @submit=${(e) => this.send(e)}>
      <yen-text-field label="Add to the checklist" .value=${this.title} @input=${(e) => (this.title = e.detail)}></yen-text-field>
      <yen-date-picker type="date" label="Due" .value=${this.due} @change=${(e) => (this.due = e.detail ?? "")}></yen-date-picker>
      <yen-button type="submit" icon="plus">Add</yen-button>
    </form>`;
  }
}
customElements.define("task-adder", TaskAdder);
