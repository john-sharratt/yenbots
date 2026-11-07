// A new card: its title, notes, tags and photo.
import { YenElement, css, go, html, toast } from "/_/yen.js";
import { make } from "../stores/items.js";

export class CollectionNew extends YenElement {
  static properties = { title: { state: true }, notes: { state: true }, tags: { state: true }, photo: { state: true }, saving: { state: true } };

  static styles = css`
    form {
      display: grid;
      gap: var(--yen-space-4);
    }
  `;

  constructor() {
    super();
    this.title = "";
    this.notes = "";
    this.tags = "";
    this.photo = null;
    this.saving = false;
  }

  async save(e) {
    e.preventDefault();
    if (!this.title.trim()) return;
    this.saving = true;
    try {
      const tags = this.tags.split(",").map((t) => t.trim()).filter(Boolean);
      const made = await make({ title: this.title.trim(), notes: this.notes, tags, photo: this.photo });
      go(made ? `i/${made.id}` : "./", { replace: true });
    } catch (err) {
      toast(err.detail ?? "Couldn't save it", { tone: "danger" });
    } finally {
      this.saving = false;
    }
  }

  render() {
    return html`<yen-page heading="A new recipe" back="./">
      <form @submit=${(e) => this.save(e)}>
        <yen-text-field label="Title" required .value=${this.title} @input=${(e) => (this.title = e.detail)}></yen-text-field>
        <yen-text-area label="Notes" rows="6" .value=${this.notes} @input=${(e) => (this.notes = e.detail)}></yen-text-area>
        <yen-text-field label="Tags" hint="Separated by commas: soup, quick" .value=${this.tags} @input=${(e) => (this.tags = e.detail)}></yen-text-field>
        <yen-image-picker label="A photo" @change=${(e) => (this.photo = e.detail[0] ?? null)}></yen-image-picker>
        <yen-button type="submit" variant="primary" ?loading=${this.saving}>Save it</yen-button>
      </form>
    </yen-page>`;
  }
}
customElements.define("collection-new", CollectionNew);
