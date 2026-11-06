// The catalogue's foundations (§8.5): every token, shown as what it does —
// colours as swatches, type as specimens, space as lengths, corners and lift
// on boxes, motion moving — and every icon. Each section is a function of
// the tokens read from `tokens.css`, so a new token appears here unasked.

import { html, styleMap } from "/_/vendor/lit-all.min.js";
import { ICONS } from "/_/ui/icons.js";
import { groupOf } from "./tokens.js";

const SURFACES = ["warm", "plain", "cool", "rose", "sage"];
const PARTS = ["bg", "surface", "surface-2", "surface-3", "line", "line-strong"];

/** The names of `group`, in the order tokens.css gives them. */
const named = (tokens, group) => tokens.light.map(([n]) => n).filter((n) => groupOf(n) === group);

const swatch = (name) => html`<div class="swatch">
  <div class="chip" style=${styleMap({ background: `var(--yen-${name})` })}></div>
  <strong>${name}</strong>
  <code>--yen-${name}</code>
</div>`;

export function colours(tokens) {
  return html`<div class="demo">
      <h3>What each colour is for</h3>
      <div class="swatches">${named(tokens, "colour").map(swatch)}</div>
    </div>
    <div class="demo">
      <h3>The palette by name: tiles, illustrations, charts</h3>
      <div class="swatches">${named(tokens, "palette").map(swatch)}</div>
    </div>`;
}

export function surfaces() {
  return html`${SURFACES.map(
    (s) => html`<div class="demo">
      <h3>${s}${s === "warm" ? " — the platform's" : ""} · <code>surface: ${s}</code></h3>
      <div class="swatches">${PARTS.map((p) => swatch(`${s}-${p}`))}</div>
    </div>`,
  )}`;
}

const SIZES = ["3xl", "2xl", "xl", "lg", "md", "sm", "xs"];
const WEIGHTS = ["regular", "medium", "semibold", "bold"];
const LINE = "Bữa cơm gia đình — the family's dinner";

export function type() {
  return html`<div class="demo">
      <h3>Headings: the display face (Fraunces), or the text face with <code>type: text</code></h3>
      <h1>Công thức của Bà — Grandma's recipes</h1>
      <h2>Nhắc việc hôm nay — Today's reminders</h2>
      <h3>Danh sách đi chợ</h3>
    </div>
    <div class="demo">
      <h3>Sizes</h3>
      ${SIZES.map(
        (s) => html`<div class="specimen">
          <code>--yen-text-${s}</code>
          <span style=${styleMap({ fontSize: `var(--yen-text-${s})`, lineHeight: "var(--yen-leading-tight)" })}>${LINE}</span>
        </div>`,
      )}
    </div>
    <div class="demo">
      <h3>The text face (Be Vietnam Pro) at its four weights</h3>
      ${WEIGHTS.map(
        (w) => html`<div class="specimen">
          <code>--yen-weight-${w}</code>
          <span style=${styleMap({ fontWeight: `var(--yen-weight-${w})` })}>${LINE}</span>
        </div>`,
      )}
    </div>
    <div class="demo">
      <h3>Body, at its measure (<code>--yen-measure</code>) and leading (<code>--yen-leading</code>)</h3>
      <p>
        Mỗi tối thứ Sáu cả nhà quây quần bên mâm cơm: canh chua cá lóc, thịt kho trứng, rau muống xào tỏi. Every Friday
        the whole family gathers around the tray — sour fish soup, caramelised pork with eggs, morning glory with garlic.
      </p>
      <p><small>Small print, in the secondary ink: <code>--yen-ink-2</code>.</small></p>
    </div>`;
}

const LENGTHS = (tokens, prefix) => tokens.light.map(([n]) => n).filter((n) => n.startsWith(prefix) && /-\d$/.test(n));

export function space(tokens) {
  return html`<div class="demo">
      <h3>The spacing scale</h3>
      ${LENGTHS(tokens, "space-").map(
        (n) => html`<div class="token">
          <code>--yen-${n}</code>
          <div class="measure-bar" style=${styleMap({ width: `var(--yen-${n})` })}></div>
        </div>`,
      )}
    </div>
    <div class="demo">
      <h3>Density: how much room a control takes (<code>density: compact | normal | roomy</code>)</h3>
      <div class="row">
        ${["compact", "normal", "roomy"].map(
          // The control's height is worked out where the density is set, so both are set here.
          (d) => html`<div
            class="state"
            style=${styleMap({
              "--yen-density": `var(--yen-density-${d})`,
              "--yen-control-height": `calc(2.75rem * var(--yen-density-${d}))`,
            })}
          >
            <span>${d}</span>
            <yen-button>Save</yen-button>
          </div>`,
        )}
      </div>
    </div>`;
}

export function corners() {
  return html`<div class="demo">
      <h3>Corners in use: <code>--yen-radius-1</code>, <code>-2</code>, <code>-3</code> and a button's</h3>
      <div class="swatches">
        ${["1", "2", "3", "button"].map(
          (r) => html`<div class="box" style=${styleMap({ borderRadius: `var(--yen-radius-${r})` })}>radius-${r}</div>`,
        )}
      </div>
    </div>
    ${["soft", "round", "square"].map(
      (set) => html`<div class="demo">
        <h3><code>radius: ${set}</code></h3>
        <div class="row">
          ${["1", "2", "3"].map(
            (r) => html`<div class="box" style=${styleMap({ width: "5rem", borderRadius: `var(--yen-radius-${set}-${r})` })}>${r}</div>`,
          )}
          <div class="state" style=${styleMap({ "--yen-radius-button": `var(--yen-radius-${set}-button)` })}>
            <yen-button>A button</yen-button>
          </div>
        </div>
      </div>`,
    )}`;
}

export function lift() {
  return html`<div class="demo">
    <h3>Lift: a card, something raised over the page, a sheet or dialog</h3>
    <div class="swatches">
      ${["1", "2", "3"].map(
        (s) => html`<div
          class="box"
          style=${styleMap({ boxShadow: `var(--yen-shadow-${s})`, borderRadius: "var(--yen-radius-2)", border: "none" })}
        >
          shadow-${s}
        </div>`,
      )}
    </div>
  </div>`;
}

export function motion(tokens) {
  const durations = tokens.light.map(([n]) => n).filter((n) => n.startsWith("duration-"));
  const eases = tokens.light.map(([n]) => n).filter((n) => n.startsWith("ease-"));
  const move = (e) => e.currentTarget.classList.toggle("moved");
  return html`<div class="demo">
    <h3>Tap a track to move it. With "reduce motion" set on the device, every one is still.</h3>
    ${durations.flatMap((d) =>
      eases.map(
        (ease) => html`<div class="token">
          <code>${d} · ${ease}</code>
          <div class="track" role="button" tabindex="0" aria-label=${`Move with ${d} and ${ease}`} @click=${move}
            @keydown=${(e) => (e.key === "Enter" || e.key === " ") && move(e)}>
            <div class="mover" style=${styleMap({ transition: `translate var(--yen-${d}) var(--yen-${ease})` })}></div>
          </div>
        </div>`,
      ),
    )}
  </div>`;
}

export function icons() {
  return html`<div class="demo">
    <h3><code>&lt;yen-icon name="…"&gt;</code>, in the ink around it</h3>
    <div class="icons">
      ${ICONS.map((name) => html`<figure><yen-icon name=${name}></yen-icon><figcaption>${name}</figcaption></figure>`)}
    </div>
  </div>`;
}
