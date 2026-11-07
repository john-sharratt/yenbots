// A card: its photo, its title, its tags. It is given its photo's address;
// a card with none shows its title alone.
import { YenElement, css, html, repeat } from "/_/yen.js";

export class ItemCard extends YenElement {
  static properties = { item: { attribute: false }, thumb: { type: String } };

  static styles = css`
    img {
      width: 100%;
      aspect-ratio: 4 / 3;
      object-fit: cover;
    }
    .tags {
      display: flex;
      flex-wrap: wrap;
      gap: var(--yen-space-1);
      margin-top: var(--yen-space-2);
    }
  `;

  render() {
    const i = this.item;
    return html`<yen-card href="i/${i.id}">
      ${this.thumb ? html`<img slot="media" src=${this.thumb} alt="">` : ""}
      <strong>${i.title}</strong>
      <div class="tags">${repeat(i.tags ?? [], (t) => t, (t) => html`<yen-tag>${t}</yen-tag>`)}</div>
    </yen-card>`;
  }
}
customElements.define("item-card", ItemCard);
