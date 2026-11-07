// The cards: every one, those with a tag, or those a search finds.
import { YenElement, css, html, repeat } from "/_/yen.js";
import { find, items, photoOf, tagged, tags } from "../stores/items.js";
import "../components/item-card.js";

export class CollectionHome extends YenElement {
  static properties = { tag: { state: true }, words: { state: true }, found: { state: true } };

  static styles = css`
    .filters {
      display: grid;
      gap: var(--yen-space-3);
      margin-bottom: var(--yen-space-5);
    }
    .tags {
      display: flex;
      flex-wrap: wrap;
      gap: var(--yen-space-2);
    }
  `;

  constructor() {
    super();
    this.tag = "";
    this.words = "";
    this.found = null;
  }

  async search(words) {
    this.words = words;
    this.found = words.trim() ? await find(words.trim()) : null;
  }

  render() {
    const shown = this.found ?? tagged(this.tag);
    return html`<yen-page heading="Recipes">
      <div class="filters">
        <yen-text-field type="search" icon="search" label="Find a recipe" .value=${this.words} @change=${(e) => this.search(e.detail)}></yen-text-field>
        <div class="tags">
          ${repeat(
            tags.value,
            (t) => t,
            (t) => html`<yen-button size="small" variant=${this.tag === t ? "primary" : "secondary"} @click=${() => (this.tag = this.tag === t ? "" : t)}>${t}</yen-button>`,
          )}
        </div>
      </div>
      <yen-list
        .items=${shown}
        .item=${(i) => html`<item-card .item=${i} thumb=${photoOf(i, { thumb: true }) ?? ""}></item-card>`}
        ?loading=${items.loading.value}
        empty=${this.found ? "Nothing matches" : "No recipes yet"}
        empty-icon="utensils"
      ></yen-list>
      ${items.mayAdd ? html`<yen-fab icon="plus" href="new">Add a recipe</yen-fab>` : ""}
    </yen-page>`;
  }
}
customElements.define("collection-home", CollectionHome);
