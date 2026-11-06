// `<yen-chart>` (§8.5): numbers as a picture — a bar for each, or a line
// through them — drawn at the width it is given, in the palette's colours,
// with the same numbers in a table a screen reader reads.
//
//   <yen-chart kind="bar" label="Spent each month" .rows=${[{label: "Jan", value: 120}, …]}
//              .format=${(v) => `$${v}`}></yen-chart>
//
// `kind` is bar (the default) or line; `.format` writes a value as words
// (en-AU numbers when absent); `empty` is what shows with no rows.

import { css, html, nothing, svg } from "../../vendor/lit-all.min.js";
import { YenElement } from "../../framework/element.js";
import { bars, line, linear, ticks } from "../scale.js";
import { base } from "../styles.js";
import "./empty.js";

const PAD = { left: 48, right: 12, top: 12, bottom: 28 };

/** The smallest room a label along the bottom takes, in the chart's pixels. */
const LABEL_ROOM = 44;

export class YenChart extends YenElement {
  static properties = {
    kind: { type: String, reflect: true },
    label: { type: String },
    rows: { attribute: false },
    format: { attribute: false },
    empty: { type: String },
    width: { state: true },
  };

  static styles = [
    base,
    css`
      :host {
        display: block;
      }
      figure {
        margin: 0;
      }
      figcaption {
        margin-bottom: var(--yen-space-2);
        font-weight: var(--yen-weight-semibold);
      }
      svg {
        display: block;
        width: 100%;
        overflow: visible;
      }
      .grid {
        stroke: var(--yen-line);
        stroke-width: 1;
      }
      .zero {
        stroke: var(--yen-line-strong);
      }
      text {
        fill: var(--yen-ink-2);
        font-size: var(--yen-text-xs);
        font-family: var(--yen-font-text);
        font-variant-numeric: tabular-nums;
      }
      .bar {
        fill: var(--yen-chart-1);
        transform-box: fill-box;
        transform-origin: center bottom;
        animation: grow var(--yen-duration-3) var(--yen-ease-out) both;
      }
      .bar.below {
        transform-origin: center top;
      }
      .line {
        fill: none;
        stroke: var(--yen-chart-1);
        stroke-width: 3;
        stroke-linecap: round;
        stroke-linejoin: round;
      }
      .area {
        fill: color-mix(in srgb, var(--yen-chart-1) 14%, transparent);
      }
      .dot {
        fill: var(--yen-surface);
        stroke: var(--yen-chart-1);
        stroke-width: 2.5;
      }
      @keyframes grow {
        from {
          transform: scaleY(0);
        }
      }
    `,
  ];

  constructor() {
    super();
    this.kind = "bar";
    this.rows = [];
    this.width = 0;
    this.empty = "Nothing to show yet";
    this.format = new Intl.NumberFormat("en-AU", { maximumFractionDigits: 2 }).format;
  }

  connectedCallback() {
    super.connectedCallback();
    this.sizer = new ResizeObserver(([entry]) => {
      const w = Math.round(entry.contentRect.width);
      if (w !== this.width) this.width = w;
    });
    this.sizer.observe(this);
  }

  disconnectedCallback() {
    super.disconnectedCallback();
    this.sizer?.disconnect();
  }

  /** The table of the same numbers, for a screen reader. */
  table(rows) {
    return html`<table class="visually-hidden">
      <caption>
        ${this.label ?? nothing}
      </caption>
      <tbody>
        ${rows.map((r) => html`<tr><th scope="row">${r.label}</th><td>${this.format(Number(r.value) || 0)}</td></tr>`)}
      </tbody>
    </table>`;
  }

  render() {
    const rows = this.rows ?? [];
    if (rows.length === 0) return html`<yen-empty icon="grid" heading=${this.empty}></yen-empty>`;
    const values = rows.map((r) => Number(r.value) || 0);
    const W = Math.max(this.width || 320, 160);
    const H = Math.round(Math.min(320, Math.max(180, W * 0.5)));
    const iw = W - PAD.left - PAD.right;
    const ih = H - PAD.top - PAD.bottom;
    const axis = ticks(Math.min(...values, 0), Math.max(...values, 0), 4);
    const y = linear([axis[0], axis[axis.length - 1]], [ih, 0]);
    const slot = iw / rows.length;
    const every = Math.max(1, Math.ceil(LABEL_ROOM / slot));
    const xOf = (i) => (this.kind === "line" ? (rows.length > 1 ? (i * iw) / (rows.length - 1) : 0) : i * slot + slot / 2);

    const grid = axis.map(
      (t) => svg`<line class=${t === 0 ? "grid zero" : "grid"} x1="0" x2=${iw} y1=${y(t)} y2=${y(t)}></line>
        <text x="-8" y=${y(t)} dy="0.32em" text-anchor="end">${this.format(t)}</text>`,
    );
    const labels = rows.map((r, i) =>
      i % every === 0 ? svg`<text x=${xOf(i)} y=${ih + 20} text-anchor="middle">${r.label}</text>` : nothing,
    );
    let marks;
    if (this.kind === "line") {
      const path = line(values, { width: iw, height: ih, axis });
      const area = `${path} L${xOf(rows.length - 1)} ${y(0)} L0 ${y(0)} Z`;
      marks = svg`<path class="area" d=${area}></path><path class="line" d=${path}></path>
        ${values.map((v, i) => svg`<circle class="dot" cx=${xOf(i)} cy=${y(v)} r="4"></circle>`)}`;
    } else {
      const shapes = bars(values, { width: iw, height: ih, axis });
      const r = Math.min(6, shapes[0].width / 4);
      marks = shapes.map(
        (b, i) => svg`<rect class=${values[i] < 0 ? "bar below" : "bar"} x=${b.x} y=${b.y} width=${b.width}
          height=${b.height} rx=${r}></rect>`,
      );
    }
    return html`<figure>
      ${this.label ? html`<figcaption aria-hidden="true">${this.label}</figcaption>` : nothing}
      <svg viewBox="0 0 ${W} ${H}" height=${H} role="img" aria-label=${this.label ?? "Chart"}>
        <g transform="translate(${PAD.left} ${PAD.top})">${grid}${marks}${labels}</g>
      </svg>
      ${this.table(rows)}
    </figure>`;
  }
}

customElements.define("yen-chart", YenChart);
