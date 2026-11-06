// `<yen-empty icon="utensils" heading="No recipes yet">` (§8.5): what a
// page, a list or a search shows when it has nothing to show — what is
// missing, a line on what to do, and the one action that fills it. With
// `art`, it shows the art library's empty-state picture of that id
// (`/_library?kind=empty`, §15) in place of the icon.
//
//   <yen-empty art="no-recipes" heading="No recipes yet">
//     Add the first, and everyone in the family will see it.
//     <yen-button slot="action" href="new">Add a recipe</yen-button>
//   </yen-empty>

import { css, html } from "../../vendor/lit-all.min.js";
import { YenElement } from "../../framework/element.js";
import { base } from "../styles.js";
import "./icon.js";

export class YenEmpty extends YenElement {
  static properties = {
    icon: { type: String },
    art: { type: String },
    heading: { type: String },
  };

  static styles = [
    base,
    css`
      :host {
        display: grid;
        /* One column as wide as the space, so long words wrap within it. */
        grid-template-columns: minmax(0, 1fr);
        justify-items: center;
        gap: var(--yen-space-3);
        padding: var(--yen-space-7) var(--yen-space-4);
        text-align: center;
      }
      .mark {
        display: grid;
        place-items: center;
        width: 4.5rem;
        height: 4.5rem;
        border-radius: 50%;
        background: var(--yen-accent-soft);
        color: var(--yen-accent);
      }
      .mark yen-icon {
        --yen-icon-size: 2rem;
      }
      .art {
        width: min(100%, 18rem);
        height: auto;
        aspect-ratio: 4 / 3;
      }
      h3 {
        margin: 0;
        font-family: var(--yen-font-heading);
        font-size: var(--yen-text-xl);
        font-weight: var(--yen-weight-semibold);
        line-height: var(--yen-leading-tight);
      }
      .words {
        max-width: 28rem;
        color: var(--yen-ink-2);
      }
      .action {
        margin-top: var(--yen-space-2);
      }
    `,
  ];

  render() {
    const mark = this.art
      ? html`<img class="art" src="/_library/empty/${this.art}-600.webp" srcset="/_library/empty/${this.art}-300.webp 300w, /_library/empty/${this.art}-600.webp 600w" sizes="18rem" width="600" height="450" alt="">`
      : html`<div class="mark"><yen-icon name=${this.icon || "sparkles"}></yen-icon></div>`;
    return html`${mark}
      <h3>${this.heading}</h3>
      <div class="words"><slot></slot></div>
      <div class="action"><slot name="action"></slot></div>`;
  }
}

customElements.define("yen-empty", YenEmpty);
