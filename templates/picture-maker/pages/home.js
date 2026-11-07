// Describe a picture and see it drawn; "Use this" keeps it.
import { YenElement, html, toast } from "/_/yen.js";
import { busy, draw, drawn, error, keep, pictures, progress } from "../stores/pictures.js";

export class PictureHome extends YenElement {
  async use(picture) {
    await keep(picture);
    toast("Kept with your pictures");
  }

  render() {
    return html`<yen-page heading="Pictures">
      <yen-button slot="actions" variant="ghost" size="small" icon="image" href="kept">Kept (${pictures.value.length})</yen-button>
      <yen-draw
        label="Describe a picture"
        placeholder="A cat in a raincoat splashing in puddles"
        .pictures=${drawn.value}
        ?busy=${busy.value}
        .progress=${progress.value ?? undefined}
        error=${error.value}
        @draw=${(e) => draw(e.detail)}
        @pick=${(e) => this.use(e.detail)}
      ></yen-draw>
    </yen-page>`;
  }
}
customElements.define("picture-home", PictureHome);
