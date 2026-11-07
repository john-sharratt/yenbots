// An entry in the list: its day, its first words, its photo.
import { YenElement, css, html } from "/_/yen.js";

export class EntryCard extends YenElement {
  static properties = { entry: { attribute: false }, day: { type: String }, thumb: { type: String } };

  static styles = css`
    p {
      margin: var(--yen-space-2) 0 0;
      color: var(--yen-ink-2);
      display: -webkit-box;
      -webkit-line-clamp: 2;
      -webkit-box-orient: vertical;
      overflow: hidden;
    }
    img {
      width: 100%;
      aspect-ratio: 3 / 2;
      object-fit: cover;
    }
  `;

  render() {
    const e = this.entry;
    return html`<yen-card href="e/${e.id}">
      ${this.thumb ? html`<img slot="media" src=${this.thumb} alt="">` : ""}
      <strong>${this.day}</strong>
      <p>${e.text}</p>
    </yen-card>`;
  }
}
customElements.define("entry-card", EntryCard);
