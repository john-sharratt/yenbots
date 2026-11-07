// One card, whole, and changing it.
import { YenElement, ask, css, go, html, repeat } from "/_/yen.js";
import { items, photoOf } from "../stores/items.js";

export class CollectionItem extends YenElement {
  static styles = css`
    img {
      width: 100%;
      border-radius: var(--yen-radius-3);
    }
    .tags {
      display: flex;
      flex-wrap: wrap;
      gap: var(--yen-space-2);
    }
    p {
      white-space: pre-wrap;
      line-height: var(--yen-leading);
    }
  `;

  async remove(i) {
    if (await ask({ heading: `Delete ${i.title}?`, confirm: "Delete", tone: "danger" })) {
      await items.remove(i.id);
      go("./", { replace: true });
    }
  }

  render() {
    const i = items.get(this.params.id);
    if (!i) {
      return html`<yen-page heading="Recipe" back="./">
        <p>${items.loading.value ? "Loading…" : "There's no such recipe."}</p>
      </yen-page>`;
    }
    const photo = photoOf(i);
    return html`<yen-page heading=${i.title} back="./">
      ${photo ? html`<img src=${photo} alt=${i.title}>` : ""}
      <div class="tags">${repeat(i.tags ?? [], (t) => t, (t) => html`<yen-tag>${t}</yen-tag>`)}</div>
      ${i.notes ? html`<p>${i.notes}</p>` : ""}
      ${items.mayEdit(i)
        ? html`<yen-form .collection=${items} .record=${i} .fields=${["title", "notes", "tags"]} submit="Save changes"></yen-form>
            <yen-button variant="danger" icon="trash" @click=${() => this.remove(i)}>Delete</yen-button>`
        : ""}
    </yen-page>`;
  }
}
customElements.define("collection-item", CollectionItem);
