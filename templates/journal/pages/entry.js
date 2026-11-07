// One entry, whole: its day, its words, its photo.
import { YenElement, ask, css, go, html } from "/_/yen.js";
import { entries, photoOf, spoken } from "../stores/entries.js";

export class JournalEntry extends YenElement {
  static styles = css`
    img {
      width: 100%;
      border-radius: var(--yen-radius-3);
    }
    p {
      white-space: pre-wrap;
      line-height: var(--yen-leading);
    }
  `;

  async remove(e) {
    if (await ask({ heading: "Delete this entry?", confirm: "Delete", tone: "danger" })) {
      await entries.remove(e.id);
      go("./", { replace: true });
    }
  }

  render() {
    const e = entries.get(this.params.id);
    if (!e) {
      return html`<yen-page heading="Entry" back="./">
        <p>${entries.loading.value ? "Loading…" : "There's no such entry."}</p>
      </yen-page>`;
    }
    const photo = photoOf(e);
    return html`<yen-page heading=${spoken(e.day)} back="./">
      ${photo ? html`<img src=${photo} alt="The photo for ${spoken(e.day)}">` : ""}
      <p>${e.text}</p>
      ${entries.mayEdit(e) ? html`<yen-button variant="danger" icon="trash" @click=${() => this.remove(e)}>Delete</yen-button>` : ""}
    </yen-page>`;
  }
}
customElements.define("journal-entry", JournalEntry);
