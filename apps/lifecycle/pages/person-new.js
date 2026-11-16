// Adding a person. The stage they start at gives them its standard checklist.
import { YenElement, go, html, toast } from "/_/yen.js";
import { EMPLOYMENT, STAGES } from "../stores/people.js";
import { addPerson } from "../stores/journey.js";
import "../components/person-editor.js";

export class PersonNew extends YenElement {
  static properties = { saving: { state: true } };

  constructor() {
    super();
    this.saving = false;
  }

  async save(fields) {
    this.saving = true;
    try {
      const made = await addPerson(fields);
      go(made ? `p/${made.id}` : "./", { replace: true });
    } catch (err) {
      toast(err.detail ?? "Couldn't save it", { tone: "danger" });
    } finally {
      this.saving = false;
    }
  }

  render() {
    return html`<yen-page heading="Add a person" back="./">
      <person-editor .employment=${EMPLOYMENT} .stages=${STAGES} submit="Add them" ?saving=${this.saving} @save=${(e) => this.save(e.detail)}></person-editor>
    </yen-page>`;
  }
}
customElements.define("person-new", PersonNew);
