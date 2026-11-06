// `/_design` (§8.5): every token and every component in every state, light
// and dark side by side, at a phone's width or a desktop's — the reference
// pages are built against, and the page to open when something looks wrong.
// `?app=<id>` shows it all in that app's theme; `?part=<id>` only that
// section (`?part=buttons`), for pointing at one thing.

import { html, nothing } from "/_/vendor/lit-all.min.js";
import { YenElement } from "/_/framework/element.js";
import "/_/framework/form.js";
import "/_/ui/components/index.js";
import { ai, buttons, data, fields, forms, layout, overlays, pictures, status } from "./components.js";
import { colours, corners, icons, lift, motion, space, surfaces, type } from "./foundations.js";
import { framed, readTokens } from "./tokens.js";

const SECTIONS = [
  ["colour", "Colour", "What each colour is for, and the palette by name.", colours],
  ["surfaces", "Surfaces", "The five an app may choose for its theme.", surfaces],
  ["type", "Type", "Two faces, both with every Vietnamese letter.", type],
  ["space", "Space and density", "", space],
  ["corners", "Corners", "", corners],
  ["lift", "Lift", "", lift],
  ["motion", "Motion", "", motion],
  ["icons", "Icons", "", icons],
  ["buttons", "Buttons", "", buttons],
  ["fields", "Fields", "", fields],
  ["form", "Forms", "", null],
  ["layout", "Cards, lists, tabs and pages", "", layout],
  ["overlays", "Sheets, dialogs and toasts", "", overlays],
  ["status", "Empty, loading and status", "", status],
  ["pictures", "Pictures", "", pictures],
  ["data", "Charts and calendars", "", data],
  ["ai", "AI and drawing", "", ai],
];

/** A collection as `<yen-form>` sees one, keeping nothing. */
const SAMPLE_COLLECTION = {
  described: {
    fields: {
      title: { type: "text", required: true, max: 120 },
      serves: { type: "number", min: 1 },
      vegetarian: { type: "bool" },
      cooked: { type: "date" },
      tags: { type: "list", of: "text" },
      method: { type: "text", long: true },
    },
  },
  add: async (fields) => ({ id: "sample", ...fields }),
  update: async (id, fields) => ({ id, ...fields }),
};

const NONE = { light: [], dark: [] };

class YenCatalogue extends YenElement {
  static properties = {
    scheme: { state: true },
    width: { state: true },
    tokens: { state: true },
    theme: { state: true },
    themed: { state: true },
    fault: { state: true },
  };

  constructor() {
    super();
    this.scheme = "both";
    this.width = matchMedia("(max-width: 48rem)").matches ? "phone" : "desktop";
    this.tokens = null;
    this.theme = NONE;
    const asked = new URLSearchParams(location.search);
    this.themed = asked.get("app");
    // Not `part`: every element has one already, the ::part() names.
    this.only = asked.get("part");
    this.fault = null;
  }

  // The catalogue's frames take its own stylesheet, so it renders in the page.
  createRenderRoot() {
    return this;
  }

  connectedCallback() {
    super.connectedCallback();
    this.load().catch((e) => (this.fault = String(e?.message ?? e)));
  }

  async load() {
    const text = async (url) => {
      const res = await fetch(url);
      if (!res.ok) throw new Error(`${url} answered ${res.status}`);
      return res.text();
    };
    this.tokens = readTokens(await text("/_/ui/tokens.css"));
    if (this.themed) {
      const href = `/_/theme/${encodeURIComponent(this.themed)}.css`;
      this.theme = readTokens(await text(href));
      const link = document.createElement("link");
      link.rel = "stylesheet";
      link.href = href;
      document.head.append(link);
    }
  }

  updated() {
    if (!this.tokens) return;
    for (const frame of this.querySelectorAll(".frame")) {
      const scheme = frame.dataset.scheme;
      if (frame.dataset.painted === `${scheme}:${this.themed ?? ""}`) continue;
      for (const [name, value] of framed(this.tokens, this.theme, scheme)) frame.style.setProperty(`--yen-${name}`, value);
      frame.style.colorScheme = scheme;
      frame.dataset.painted = `${scheme}:${this.themed ?? ""}`;
    }
  }

  choice(label, value, options, set) {
    return html`<div class="choice" role="group" aria-label=${label}>
      ${options.map(
        ([v, words]) => html`<button type="button" aria-pressed=${v === value ? "true" : "false"} @click=${() => set(v)}>${words}</button>`,
      )}
    </div>`;
  }

  frames(id, show) {
    const schemes = this.scheme === "both" ? ["light", "dark"] : [this.scheme];
    return html`<div class="frames ${this.width}">
      ${schemes.map(
        (s) => html`<div class="frame" data-scheme=${s}>
          <p class="scheme">${s}</p>
          ${show()}
        </div>`,
      )}
    </div>`;
  }

  render() {
    if (this.fault) return html`<yen-empty icon="alert" heading="The catalogue couldn't load">${this.fault}</yen-empty>`;
    if (!this.tokens) return html`<yen-progress kind="spinner" label="Loading"></yen-progress>`;
    return html`<div class="bar">
        <h1>
          The design system
          <small>${this.themed ? html`In the theme of <strong>${this.themed}</strong>` : "Yen Bots' own look"} ·
            <a href="/_design/vietnamese">Vietnamese test page</a></small>
        </h1>
        ${this.choice("Light or dark", this.scheme, [["both", "Both"], ["light", "Light"], ["dark", "Dark"]], (v) => (this.scheme = v))}
        ${this.choice("Width", this.width, [["phone", "Phone"], ["desktop", "Desktop"]], (v) => (this.width = v))}
      </div>
      <nav class="sections" aria-label="Sections">
        ${SECTIONS.map(([id, title]) => html`<a href=${this.only ? `?part=${id}` : `#${id}`}>${title}</a>`)}
        ${this.only ? html`<a href="/_design">Everything</a>` : nothing}
      </nav>
      ${SECTIONS.filter(([id]) => !this.only || id === this.only).map(
        ([id, title, about, show]) => html`<section class="part" id=${id}>
          <h2>${title}</h2>
          ${about ? html`<p>${about}</p>` : nothing}
          ${this.frames(id, () => (show ? show(this.tokens) : forms(SAMPLE_COLLECTION)))}
        </section>`,
      )}
      <footer class="end">Every component lives in <code>web/_/ui/components/</code>, one to a file, named for its tag.</footer>`;
  }
}

customElements.define("yen-catalogue", YenCatalogue);
