// The pictures kept.
import { YenElement, html } from "/_/yen.js";
import { kept, pictures } from "../stores/pictures.js";

export class PictureKept extends YenElement {
  render() {
    return html`<yen-page heading="Kept" back="./">
      ${pictures.loading.value && kept.value.length === 0
        ? html`<yen-skeleton shape="block"></yen-skeleton>`
        : kept.value.length === 0
          ? html`<yen-empty icon="image" heading="Nothing kept yet">Draw a picture, then choose "Use this".</yen-empty>`
          : html`<yen-photo-grid .photos=${kept.value} size="medium"></yen-photo-grid>`}
    </yen-page>`;
  }
}
customElements.define("picture-kept", PictureKept);
