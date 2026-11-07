// Something on a day.
import { YenElement, go, html } from "/_/yen.js";
import { items } from "../stores/items.js";

export class DigestNew extends YenElement {
  render() {
    return html`<yen-page heading="Something on" back="./">
      <yen-form .collection=${items} .fields=${["title", "due_ms"]} submit="Add it" @saved=${() => go("./", { replace: true })}></yen-form>
    </yen-page>`;
  }
}
customElements.define("digest-new", DigestNew);
