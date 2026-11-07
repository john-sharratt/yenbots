// One item: change its words, or take it off the list.
import { YenElement, ask, go, html } from "/_/yen.js";
import { items } from "../stores/items.js";

export class ListItemPage extends YenElement {
  async remove(item) {
    if (await ask({ heading: `Take ${item.text} off the list?`, confirm: "Take it off", tone: "danger" })) {
      await items.remove(item.id);
      go("./", { replace: true });
    }
  }

  render() {
    const item = items.get(this.params.id);
    if (!item) {
      return html`<yen-page heading="Item" back="./">
        <p>${items.loading.value ? "Loading…" : "That item is not on the list any more."}</p>
      </yen-page>`;
    }
    return html`<yen-page heading=${item.text} back="./">
      <yen-form .collection=${items} .record=${item} .fields=${["text"]} @saved=${() => go("./")}></yen-form>
      ${items.mayEdit(item)
        ? html`<yen-button variant="danger" icon="trash" @click=${() => this.remove(item)}>Take it off the list</yen-button>`
        : ""}
    </yen-page>`;
  }
}
customElements.define("list-item-page", ListItemPage);
