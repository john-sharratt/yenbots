// The list: add to it, tick things off, put them in order.
import { YenElement, css, html } from "/_/yen.js";
import { add, items, move, ordered, toggle } from "../stores/items.js";
import "../components/list-row.js";

export class ListHome extends YenElement {
  static properties = { draft: { state: true } };

  static styles = css`
    form {
      display: flex;
      gap: var(--yen-space-3);
      align-items: end;
      margin-bottom: var(--yen-space-5);
    }
    yen-text-field {
      flex: 1;
    }
  `;

  constructor() {
    super();
    this.draft = "";
  }

  async submit(e) {
    e.preventDefault();
    const text = this.draft.trim();
    if (!text) return;
    this.draft = "";
    await add(text);
  }

  render() {
    const shown = ordered.value;
    const open = shown.filter((i) => !i.done);
    return html`<yen-page heading="Shopping">
      ${items.mayAdd
        ? html`<form @submit=${(e) => this.submit(e)}>
            <yen-text-field label="Add to the list" .value=${this.draft} @input=${(e) => (this.draft = e.detail)}></yen-text-field>
            <yen-button type="submit" icon="plus">Add</yen-button>
          </form>`
        : ""}
      <yen-list
        divided
        .items=${shown}
        .item=${(i) => html`<list-row
          .item=${i}
          ?first=${open[0]?.id === i.id}
          ?last=${open[open.length - 1]?.id === i.id}
          @toggle=${(e) => toggle(e.detail)}
          @up=${(e) => move(e.detail, -1)}
          @down=${(e) => move(e.detail, 1)}
        ></list-row>`}
        ?loading=${items.loading.value}
        empty="The list is empty"
        empty-icon="cart"
      ></yen-list>
    </yen-page>`;
  }
}
customElements.define("list-home", ListHome);
