// `<yen-avatar name="Nguyễn Thị Yến" src=${picture}>` (§8.5): a person's
// picture in a circle — or, with none or one that will not load, their
// initials on their own colour, the same on every page. `size` is small,
// medium (the default) or large.

import { css, html, nothing } from "../../vendor/lit-all.min.js";
import { YenElement } from "../../framework/element.js";
import { initials } from "../initials.js";
import { paletteOf } from "../palette.js";
import { base } from "../styles.js";

export class YenAvatar extends YenElement {
  static properties = {
    name: { type: String },
    src: { type: String },
    size: { type: String, reflect: true },
    broken: { state: true },
  };

  static styles = [
    base,
    css`
      :host {
        --size: 2.5rem;
        display: inline-grid;
        flex: none;
        width: var(--size);
        height: var(--size);
      }
      :host([size="small"]) {
        --size: 2rem;
      }
      :host([size="large"]) {
        --size: 4rem;
      }
      .face {
        display: grid;
        place-items: center;
        width: 100%;
        height: 100%;
        border-radius: 50%;
        overflow: hidden;
        background: color-mix(in srgb, var(--hue) 30%, var(--yen-surface));
        color: var(--yen-ink);
        font-weight: var(--yen-weight-semibold);
        font-size: calc(var(--size) * 0.38);
        line-height: 1;
        letter-spacing: 0.02em;
        user-select: none;
        box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--yen-ink) 8%, transparent);
      }
      img {
        width: 100%;
        height: 100%;
        object-fit: cover;
      }
    `,
  ];

  constructor() {
    super();
    this.broken = false;
    this.size = "medium";
  }

  updated(changed) {
    if (changed.has("src")) this.broken = false;
    // The colour is a token by name, set as a property, never inline markup.
    const hue = `var(--yen-${paletteOf(this.name)})`;
    if (this.style.getPropertyValue("--hue") !== hue) this.style.setProperty("--hue", hue);
  }

  render() {
    const picture = this.src && !this.broken;
    return html`<span class="face" role="img" aria-label=${this.name ?? nothing}>
      ${picture
        ? html`<img src=${this.src} alt="" referrerpolicy="no-referrer" @error=${() => (this.broken = true)} />`
        : html`<span aria-hidden="true">${initials(this.name)}</span>`}
    </span>`;
  }
}

customElements.define("yen-avatar", YenAvatar);
