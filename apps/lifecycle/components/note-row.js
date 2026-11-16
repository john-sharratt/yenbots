// A file note: its day, its kind, the matter it belongs to if any, and its
// words. It raises `remove`.
import { YenElement, css, html } from "/_/yen.js";

export class NoteRow extends YenElement {
  static properties = { note: { attribute: false }, day: { type: String }, matter: { type: String } };

  static styles = css`
    :host {
      display: block;
      padding: var(--yen-space-3);
    }
    .head {
      display: flex;
      flex-wrap: wrap;
      align-items: center;
      gap: var(--yen-space-2);
    }
    .head strong {
      flex: 1;
    }
    p {
      white-space: pre-wrap;
      line-height: var(--yen-leading);
      margin-top: var(--yen-space-2);
      margin-bottom: var(--yen-space-1);
    }
  `;

  render() {
    const n = this.note;
    return html`<div class="head">
        <strong>${this.day}</strong>
        ${n.kind ? html`<yen-tag>${n.kind}</yen-tag>` : ""}
        ${this.matter ? html`<yen-tag tone="warning" icon="alert">${this.matter}</yen-tag>` : ""}
        <yen-icon-button icon="trash" label="Delete this note" variant="ghost" @click=${() => this.emit("remove", n)}></yen-icon-button>
      </div>
      <p>${n.text}${n.pending ? " (not sent yet)" : ""}</p>`;
  }
}
customElements.define("note-row", NoteRow);
