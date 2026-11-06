// `<yen-draw>` (§8.5): ask for a picture in words and see it drawn — how far
// the drawing has got while it is made, then the pictures, each with "Use
// this". The page asks the AI (`app.ai`, §14) and holds the pictures; the
// panel raises `draw` with the words and `pick` with the chosen picture.
//
//   <yen-draw .pictures=${this.drawn} ?busy=${this.drawing} .progress=${this.done}
//             placeholder="A bowl of phở, watercolour" @draw=${(e) => this.draw(e.detail)}
//             @pick=${(e) => this.use(e.detail)}></yen-draw>
//
// A picture is `{src, alt}`; `progress` is 0 to 1 while `busy`, or absent
// when how far is not known.

import { css, html, nothing, repeat } from "../../vendor/lit-all.min.js";
import { YenElement } from "../../framework/element.js";
import { base, button, control, field } from "../styles.js";
import "./icon.js";
import "./progress.js";
import "./skeleton.js";

export class YenDraw extends YenElement {
  static properties = {
    pictures: { attribute: false },
    busy: { type: Boolean },
    progress: { type: Number },
    placeholder: { type: String },
    label: { type: String },
    error: { type: String },
    prompt: { state: true },
  };

  static styles = [
    base,
    button,
    control,
    field,
    css`
      :host {
        display: grid;
        gap: var(--yen-space-4);
      }
      form {
        display: grid;
        gap: var(--yen-space-3);
      }
      .row {
        display: flex;
        justify-content: flex-end;
      }
      ul {
        list-style: none;
        margin: 0;
        padding: 0;
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(min(12rem, 100%), 1fr));
        gap: var(--yen-space-3);
      }
      li {
        display: grid;
        gap: var(--yen-space-2);
        animation: in var(--yen-duration-3) var(--yen-ease-out);
      }
      img,
      .canvas {
        width: 100%;
        aspect-ratio: 1;
        object-fit: cover;
        border-radius: var(--yen-radius-3);
        background: var(--yen-surface-2);
      }
      .canvas {
        display: grid;
        place-items: center;
        align-content: center;
        gap: var(--yen-space-3);
        padding: var(--yen-space-5);
        color: var(--yen-ink-2);
        text-align: center;
      }
      .canvas yen-icon {
        --yen-icon-size: 2rem;
        color: var(--yen-accent);
      }
      .canvas yen-progress {
        width: 100%;
      }
      @keyframes in {
        from {
          opacity: 0;
          transform: scale(0.96);
        }
      }
    `,
  ];

  constructor() {
    super();
    this.pictures = [];
    this.busy = false;
    this.prompt = "";
    this.placeholder = "What should it draw?";
    this.label = "Draw a picture";
  }

  draw(e) {
    e.preventDefault();
    const words = this.prompt.trim();
    if (!words || this.busy) return;
    this.emit("draw", words);
  }

  render() {
    const pictures = this.pictures ?? [];
    return html`<form @submit=${this.draw}>
        <div class="field">
          <label class="label" for="prompt">${this.label}</label>
          <textarea
            class="control"
            id="prompt"
            rows="2"
            placeholder=${this.placeholder}
            .value=${this.prompt}
            aria-invalid=${this.error ? "true" : "false"}
            @input=${(e) => {
              e.stopPropagation();
              this.prompt = e.target.value;
            }}
          ></textarea>
          ${this.error ? html`<p class="error" role="alert"><yen-icon name="alert"></yen-icon>${this.error}</p>` : nothing}
        </div>
        <div class="row">
          <button class="btn primary" type="submit" ?disabled=${this.busy || !this.prompt.trim()} aria-busy=${this.busy ? "true" : "false"}>
            <yen-icon name="wand"></yen-icon>${this.busy ? "Drawing…" : "Draw it"}
          </button>
        </div>
      </form>
      ${this.busy || pictures.length
        ? html`<ul>
            ${this.busy
              ? html`<li>
                  <div class="canvas" role="status">
                    <yen-icon name="sparkles"></yen-icon>
                    <span>Drawing your picture…</span>
                    <yen-progress .value=${this.progress} label="Drawing"></yen-progress>
                  </div>
                </li>`
              : nothing}
            ${repeat(
              pictures,
              (p, i) => p.src ?? i,
              (p, i) => html`<li>
                <img src=${p.src} alt=${p.alt ?? ""} />
                <button class="btn secondary small" type="button" @click=${() => this.emit("pick", p)}>
                  <yen-icon name="check"></yen-icon>Use this
                </button>
              </li>`,
            )}
          </ul>`
        : nothing}`;
  }
}

customElements.define("yen-draw", YenDraw);
