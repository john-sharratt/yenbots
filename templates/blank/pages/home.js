// The one page: write a note, see every note, remove one.
import { YenElement, css, html, toast } from "/_/yen.js";
import { notes } from "../stores/notes.js";

export class BlankHome extends YenElement {
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

  async add(e) {
    e.preventDefault();
    const text = this.draft.trim();
    if (!text) return;
    this.draft = "";
    await notes.add({ text });
  }

  async remove(note) {
    await notes.remove(note.id);
    toast("Note removed");
  }

  render() {
    return html`<yen-page heading="Notes">
      <form @submit=${(e) => this.add(e)}>
        <yen-text-field
          label="A new note"
          .value=${this.draft}
          @input=${(e) => (this.draft = e.detail)}
        ></yen-text-field>
        <yen-button type="submit" icon="plus">Add</yen-button>
      </form>
      <yen-list
        divided
        .items=${notes.value}
        .item=${(n) => html`<yen-list-item headline=${n.text}>
          <yen-icon-button slot="end" icon="trash" label="Remove this note" @click=${() => this.remove(n)}></yen-icon-button>
        </yen-list-item>`}
        ?loading=${notes.loading.value}
        empty="No notes yet"
        empty-icon="pencil"
      ></yen-list>
    </yen-page>`;
  }
}
customElements.define("blank-home", BlankHome);
