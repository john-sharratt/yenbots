// One item of the list: its tick box, its words, and moving it up or down.
// It raises `toggle`, `up` and `down`; the page does the writing.
import { YenElement, css, html } from "/_/yen.js";

export class ListRow extends YenElement {
  static properties = { item: { attribute: false }, first: { type: Boolean }, last: { type: Boolean } };

  static styles = css`
    :host {
      display: flex;
      align-items: center;
      gap: var(--yen-space-3);
      padding: var(--yen-space-2) var(--yen-space-3);
    }
    a {
      flex: 1;
      color: var(--yen-ink);
      text-decoration: none;
    }
    .done a {
      color: var(--yen-ink-2);
      text-decoration: line-through;
    }
  `;

  render() {
    const i = this.item;
    return html`<div class=${i.done ? "done" : ""}>
        <yen-checkbox
          label=${i.done ? `Put back ${i.text}` : `Tick off ${i.text}`}
          ?checked=${i.done}
          @change=${() => this.emit("toggle", i)}
        ></yen-checkbox>
      </div>
      <a href="i/${i.id}">${i.text}${i.pending ? " (not sent yet)" : ""}</a>
      ${i.done
        ? ""
        : html`<yen-icon-button icon="chevron-up" label="Move up" ?disabled=${this.first} @click=${() => this.emit("up", i)}></yen-icon-button>
            <yen-icon-button icon="chevron-down" label="Move down" ?disabled=${this.last} @click=${() => this.emit("down", i)}></yen-icon-button>`}`;
  }
}
customElements.define("list-row", ListRow);
