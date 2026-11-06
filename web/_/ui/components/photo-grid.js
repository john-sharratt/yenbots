// `<yen-photo-grid .photos=${[{src, thumb, alt}]}>` (§8.5): pictures in
// square tiles; a tap opens one large, with the arrows (and the keys) to go
// through the rest and Escape to go back. `thumb` is the small picture the
// tile shows, `src` the large one; `alt` says what each shows. It raises
// `open` with the index of the picture opened.

import { css, html, nothing } from "../../vendor/lit-all.min.js";
import { YenElement } from "../../framework/element.js";
import { base } from "../styles.js";
import "./icon-button.js";

export class YenPhotoGrid extends YenElement {
  static properties = {
    photos: { attribute: false },
    size: { type: String, reflect: true },
    shown: { state: true },
  };

  static styles = [
    base,
    css`
      :host {
        --tile: 7rem;
        display: block;
      }
      :host([size="large"]) {
        --tile: 11rem;
      }
      ul {
        list-style: none;
        margin: 0;
        padding: 0;
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(var(--tile), 1fr));
        gap: var(--yen-space-1);
      }
      button.tile {
        display: block;
        width: 100%;
        aspect-ratio: 1;
        padding: 0;
        border: none;
        border-radius: var(--yen-radius-1);
        overflow: hidden;
        background: var(--yen-surface-3);
        cursor: zoom-in;
      }
      button.tile img {
        width: 100%;
        height: 100%;
        object-fit: cover;
        transition: transform var(--yen-duration-3) var(--yen-ease-out);
      }
      button.tile:hover img,
      :host([demo~="hover"]) button.tile img {
        transform: scale(1.04);
      }
      dialog {
        width: 100vw;
        max-width: 100vw;
        height: 100dvh;
        max-height: 100dvh;
        margin: 0;
        padding: 0;
        border: none;
        background: var(--yen-overlay-strong);
        color: var(--yen-on-overlay);
      }
      dialog[open] {
        display: grid;
        grid-template-rows: auto 1fr auto;
        animation: fade var(--yen-duration-2) var(--yen-ease-out);
      }
      .bar {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: var(--yen-space-2);
        color: var(--yen-on-overlay);
        --yen-ink: var(--yen-on-overlay);
        --yen-ink-2: var(--yen-on-overlay);
        --yen-surface-3: color-mix(in srgb, var(--yen-on-overlay) 16%, transparent);
      }
      .count {
        padding-left: var(--yen-space-3);
        font-variant-numeric: tabular-nums;
      }
      figure {
        margin: 0;
        display: grid;
        place-items: center;
        min-height: 0;
        padding: 0 var(--yen-space-2);
      }
      figure img {
        max-width: 100%;
        max-height: 100%;
        object-fit: contain;
        border-radius: var(--yen-radius-1);
      }
      figcaption {
        padding: var(--yen-space-3) var(--yen-space-4) max(var(--yen-space-4), env(safe-area-inset-bottom));
        text-align: center;
        color: var(--yen-on-overlay);
      }
      @keyframes fade {
        from {
          opacity: 0;
        }
      }
    `,
  ];

  constructor() {
    super();
    this.photos = [];
    this.shown = null;
    this.size = "medium";
  }

  open(i) {
    this.shown = i;
    this.emit("open", i);
    this.updateComplete.then(() => {
      const d = this.renderRoot.querySelector("dialog");
      if (d && !d.open) d.showModal();
    });
  }

  step(by) {
    const n = this.photos.length;
    this.shown = (((this.shown ?? 0) + by) % n + n) % n;
  }

  keyed(e) {
    if (e.key === "ArrowRight") this.step(1);
    else if (e.key === "ArrowLeft") this.step(-1);
    else return;
    e.preventDefault();
  }

  render() {
    const photos = this.photos ?? [];
    const p = this.shown === null ? null : photos[this.shown];
    return html`<ul>
        ${photos.map(
          (ph, i) => html`<li>
            <button class="tile" type="button" aria-label=${`Open ${ph.alt || `picture ${i + 1}`}`} @click=${() => this.open(i)}>
              <img src=${ph.thumb || ph.src} alt="" loading="lazy" decoding="async" />
            </button>
          </li>`,
        )}
      </ul>
      ${p
        ? html`<dialog aria-label="Pictures" @close=${() => (this.shown = null)} @keydown=${this.keyed}>
            <div class="bar">
              <span class="count">${this.shown + 1} of ${photos.length}</span>
              <yen-icon-button icon="x" label="Close" @click=${() => this.renderRoot.querySelector("dialog").close()}></yen-icon-button>
            </div>
            <figure>
              <img src=${p.src} alt=${p.alt ?? ""} />
            </figure>
            <div class="bar">
              <yen-icon-button icon="chevron-left" label="Previous" ?disabled=${photos.length < 2} @click=${() => this.step(-1)}></yen-icon-button>
              ${p.alt ? html`<figcaption>${p.alt}</figcaption>` : nothing}
              <yen-icon-button icon="chevron-right" label="Next" ?disabled=${photos.length < 2} @click=${() => this.step(1)}></yen-icon-button>
            </div>
          </dialog>`
        : nothing}`;
  }
}

customElements.define("yen-photo-grid", YenPhotoGrid);
