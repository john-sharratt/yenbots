// `<yen-image-picker label="Photos" multiple>` (§8.5): choose pictures — from
// the phone's photos, its camera, or dropped on a computer — and see them
// at once, before anything is sent. A file of the wrong kind, or too large,
// is refused beside the picker with why. The page sends the chosen files
// where they belong; the picker only chooses.
//
// `accept` is the kinds taken (`image/*` when absent), `max-bytes` the
// largest, `multiple` several at once. It raises `change` with the `File`s
// chosen, in order, after each pick or removal; `.files` is the same list.

import { css, html, nothing, repeat } from "../../vendor/lit-all.min.js";
import { YenElement } from "../../framework/element.js";
import { refusal } from "../accepts.js";
import { base, button, field } from "../styles.js";
import "./icon.js";

let made = 0;

export class YenImagePicker extends YenElement {
  static properties = {
    label: { type: String },
    hint: { type: String },
    error: { type: String },
    accept: { type: String },
    maxBytes: { type: Number, attribute: "max-bytes" },
    multiple: { type: Boolean },
    disabled: { type: Boolean, reflect: true },
    picked: { state: true },
    refused: { state: true },
    over: { state: true },
  };

  static styles = [
    base,
    button,
    field,
    css`
      :host {
        display: block;
      }
      .zone {
        display: grid;
        justify-items: center;
        gap: var(--yen-space-2);
        padding: var(--yen-space-5) var(--yen-space-4);
        border: 2px dashed var(--yen-line-strong);
        border-radius: var(--yen-radius-3);
        background: var(--yen-surface-2);
        text-align: center;
        color: var(--yen-ink-2);
        transition:
          border-color var(--yen-duration-2) var(--yen-ease-out),
          background-color var(--yen-duration-2) var(--yen-ease-out);
      }
      .zone.over,
      :host([demo~="hover"]) .zone {
        border-color: var(--yen-accent);
        background: var(--yen-accent-soft);
      }
      .zone > yen-icon {
        --yen-icon-size: 2rem;
        color: var(--yen-accent);
      }
      .buttons {
        display: flex;
        flex-wrap: wrap;
        justify-content: center;
        gap: var(--yen-space-2);
      }
      input[type="file"] {
        display: none;
      }
      ul {
        list-style: none;
        margin: var(--yen-space-3) 0 0;
        padding: 0;
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(6rem, 1fr));
        gap: var(--yen-space-2);
      }
      li {
        position: relative;
        aspect-ratio: 1;
        border-radius: var(--yen-radius-2);
        overflow: hidden;
        background: var(--yen-surface-3);
        animation: in var(--yen-duration-3) var(--yen-ease-spring);
      }
      li img {
        width: 100%;
        height: 100%;
        object-fit: cover;
      }
      .remove {
        position: absolute;
        top: var(--yen-space-1);
        right: var(--yen-space-1);
        display: grid;
        place-items: center;
        width: 2rem;
        height: 2rem;
        padding: 0;
        border: none;
        border-radius: 50%;
        background: var(--yen-overlay);
        color: var(--yen-on-overlay);
        cursor: pointer;
      }
      .remove yen-icon {
        --yen-icon-size: 1rem;
      }
      @keyframes in {
        from {
          opacity: 0;
          transform: scale(0.9);
        }
      }
    `,
  ];

  constructor() {
    super();
    this.uid = `yen-picker-${++made}`;
    this.accept = "image/*";
    this.multiple = false;
    this.disabled = false;
    this.picked = [];
    this.refused = [];
    this.over = false;
  }

  /** The files chosen, in order. */
  get files() {
    return this.picked.map((p) => p.file);
  }

  disconnectedCallback() {
    super.disconnectedCallback();
    for (const p of this.picked) URL.revokeObjectURL(p.url);
  }

  take(list) {
    const refused = [];
    const fresh = [];
    for (const file of list) {
      const why = refusal(file, { accept: this.accept, maxBytes: this.maxBytes });
      if (why) refused.push(`${file.name} ${why}.`);
      else fresh.push({ file, url: URL.createObjectURL(file), key: `${file.name}-${file.size}-${file.lastModified}` });
    }
    this.refused = refused;
    if (fresh.length === 0) return;
    if (this.multiple) {
      this.picked = [...this.picked, ...fresh];
    } else {
      for (const p of this.picked) URL.revokeObjectURL(p.url);
      this.picked = fresh.slice(0, 1);
    }
    this.emit("change", this.files);
  }

  remove(p) {
    URL.revokeObjectURL(p.url);
    this.picked = this.picked.filter((x) => x !== p);
    this.emit("change", this.files);
  }

  /** Open the device's chooser: `camera` asks a phone for its camera. */
  choose(camera) {
    const input = this.renderRoot.querySelector(camera ? "#camera" : "#files");
    input.value = "";
    input.click();
  }

  dropped(e) {
    e.preventDefault();
    this.over = false;
    if (!this.disabled) this.take([...(e.dataTransfer?.files ?? [])]);
  }

  render() {
    const errors = [...this.refused, ...(this.error ? [this.error] : [])];
    return html`<div class="field">
      ${this.label ? html`<span class="label" id=${`${this.uid}-label`}>${this.label}</span>` : nothing}
      <div
        class="zone ${this.over ? "over" : ""}"
        role="group"
        aria-labelledby=${this.label ? `${this.uid}-label` : nothing}
        @dragover=${(e) => {
          e.preventDefault();
          this.over = true;
        }}
        @dragleave=${() => (this.over = false)}
        @drop=${this.dropped}
      >
        <yen-icon name="image"></yen-icon>
        <span>${this.multiple ? "Choose pictures, or drop them here" : "Choose a picture, or drop it here"}</span>
        <div class="buttons">
          <button class="btn secondary small" type="button" ?disabled=${this.disabled} @click=${() => this.choose(false)}>
            <yen-icon name="upload"></yen-icon>Choose
          </button>
          <button class="btn ghost small" type="button" ?disabled=${this.disabled} @click=${() => this.choose(true)}>
            <yen-icon name="camera"></yen-icon>Take a photo
          </button>
        </div>
        <input id="files" type="file" accept=${this.accept} ?multiple=${this.multiple} @change=${(e) => this.take([...e.target.files])} />
        <input id="camera" type="file" accept="image/*" capture="environment" @change=${(e) => this.take([...e.target.files])} />
      </div>
      ${this.hint ? html`<p class="hint">${this.hint}</p>` : nothing}
      ${errors.map((e) => html`<p class="error" role="alert"><yen-icon name="alert"></yen-icon>${e}</p>`)}
      ${this.picked.length
        ? html`<ul>
            ${repeat(
              this.picked,
              (p) => p.key,
              (p) => html`<li>
                <img src=${p.url} alt=${p.file.name} />
                <button class="remove" type="button" aria-label=${`Remove ${p.file.name}`} @click=${() => this.remove(p)}>
                  <yen-icon name="x"></yen-icon>
                </button>
              </li>`,
            )}
          </ul>`
        : nothing}
    </div>`;
  }
}

customElements.define("yen-image-picker", YenImagePicker);
