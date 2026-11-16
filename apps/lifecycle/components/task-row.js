// A checklist item: its tick box, its words, when it is due, and removing
// it. It raises `toggle` and `remove`; the page does the writing.
import { YenElement, css, html } from "/_/yen.js";

export class TaskRow extends YenElement {
  static properties = { task: { attribute: false }, due: { type: String }, late: { type: Boolean } };

  static styles = css`
    :host {
      display: flex;
      align-items: center;
      gap: var(--yen-space-3);
      padding: var(--yen-space-2) var(--yen-space-3);
    }
    .words {
      flex: 1;
      display: grid;
      gap: var(--yen-space-1);
    }
    .due {
      color: var(--yen-ink-2);
      font-size: var(--yen-text-sm);
    }
    .late {
      color: var(--yen-danger);
    }
    .done .title {
      color: var(--yen-ink-2);
      text-decoration: line-through;
    }
  `;

  render() {
    const t = this.task;
    return html`<yen-checkbox ?checked=${t.done} @change=${() => this.emit("toggle", t)}></yen-checkbox>
      <div class="words ${t.done ? "done" : ""}">
        <span class="title">${t.title}${t.pending ? " (not sent yet)" : ""}</span>
        ${this.due ? html`<span class="due ${this.late && !t.done ? "late" : ""}">${this.due}</span>` : ""}
      </div>
      <yen-icon-button icon="trash" label="Remove ${t.title}" variant="ghost" @click=${() => this.emit("remove", t)}></yen-icon-button>`;
  }
}
customElements.define("task-row", TaskRow);
