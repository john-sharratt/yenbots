// `<yen-calendar>` (§8.5): a month, Monday first, to see what falls when and
// to choose a day. A dot marks a day with something on it; today is ringed;
// the chosen day is filled. The arrows, Home, End, Page Up and Page Down
// move between days as a date picker's do.
//
//   <yen-calendar .value=${this.day} .marks=${due} @change=${(e) => (this.day = e.detail)}></yen-calendar>
//
// `value` and every mark are `YYYY-MM-DD`. It raises `change` with the day
// chosen, and `month` with `{year, month}` when the month shown changes.

import { css, html } from "../../vendor/lit-all.min.js";
import { YenElement } from "../../framework/element.js";
import { calendarMove, isoDate, parts, shift, weeksOf } from "../month.js";
import { base } from "../styles.js";
import "./icon-button.js";

const WEEKDAYS = [
  ["Mon", "Monday"],
  ["Tue", "Tuesday"],
  ["Wed", "Wednesday"],
  ["Thu", "Thursday"],
  ["Fri", "Friday"],
  ["Sat", "Saturday"],
  ["Sun", "Sunday"],
];

/** Today on this device, `YYYY-MM-DD`. */
function today() {
  const d = new Date();
  return isoDate(d.getFullYear(), d.getMonth() + 1, d.getDate());
}

export class YenCalendar extends YenElement {
  static properties = {
    value: { type: String },
    marks: { attribute: false },
    shown: { state: true },
    focused: { state: true },
  };

  static styles = [
    base,
    css`
      :host {
        display: block;
        max-width: 26rem;
      }
      header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        margin-bottom: var(--yen-space-2);
      }
      h3 {
        margin: 0;
        font-family: var(--yen-font-heading);
        font-size: var(--yen-text-lg);
        font-weight: var(--yen-weight-semibold);
      }
      table {
        width: 100%;
        border-collapse: collapse;
        table-layout: fixed;
      }
      th {
        padding-bottom: var(--yen-space-2);
        font-size: var(--yen-text-xs);
        font-weight: var(--yen-weight-semibold);
        color: var(--yen-ink-2);
        text-transform: uppercase;
        letter-spacing: 0.06em;
      }
      abbr {
        text-decoration: none;
      }
      td {
        padding: 2px;
        text-align: center;
      }
      button.day {
        position: relative;
        width: 100%;
        max-width: var(--yen-tap);
        aspect-ratio: 1;
        padding: 0;
        border: none;
        border-radius: 50%;
        background: transparent;
        color: var(--yen-ink);
        font: inherit;
        font-variant-numeric: tabular-nums;
        cursor: pointer;
        transition: background-color var(--yen-duration-1) var(--yen-ease-out);
      }
      button.day:hover {
        background: var(--yen-surface-3);
      }
      button.day.outside {
        color: var(--yen-ink-3);
      }
      button.day.today {
        box-shadow: inset 0 0 0 2px var(--yen-accent);
        font-weight: var(--yen-weight-bold);
      }
      button.day[aria-pressed="true"] {
        background: var(--yen-accent);
        color: var(--yen-on-accent);
        font-weight: var(--yen-weight-bold);
      }
      button.day.marked::after {
        content: "";
        position: absolute;
        left: 50%;
        bottom: 14%;
        width: 0.3rem;
        height: 0.3rem;
        margin-left: -0.15rem;
        border-radius: 50%;
        background: currentColor;
        opacity: 0.85;
      }
      button.day.marked:not([aria-pressed="true"])::after {
        background: var(--yen-accent);
      }
    `,
  ];

  constructor() {
    super();
    this.marks = [];
    this.shown = null;
    this.focused = null;
  }

  /** The month shown: the chosen day's, or today's, until moved. */
  month() {
    if (this.shown) return this.shown;
    const { year, month } = parts(this.value || today());
    return { year, month };
  }

  go(by) {
    const m = this.month();
    this.shown = shift(m.year, m.month, by);
    this.emit("month", this.shown);
  }

  choose(date) {
    this.value = date;
    this.focused = date;
    this.emit("change", date);
  }

  keyed(e, date) {
    const to = calendarMove(date, e.key);
    if (to === null) return;
    e.preventDefault();
    this.focused = to;
    const p = parts(to);
    const m = this.month();
    if (p.year !== m.year || p.month !== m.month) {
      this.shown = { year: p.year, month: p.month };
      this.emit("month", this.shown);
    }
    this.updateComplete.then(() => this.renderRoot.querySelector(`[data-date="${to}"]`)?.focus());
  }

  render() {
    const { year, month } = this.month();
    const now = today();
    const marked = new Set(this.marks ?? []);
    const weeks = weeksOf(year, month);
    const inMonth = weeks.flat().filter((d) => d.inMonth).map((d) => d.date);
    // The one day the Tab key lands on.
    const tabbable = [this.focused, this.value, now].find((d) => d && inMonth.includes(d)) ?? inMonth[0];
    const title = new Intl.DateTimeFormat("en-AU", { month: "long", year: "numeric", timeZone: "UTC" }).format(
      Date.UTC(year, month - 1, 1),
    );
    const long = new Intl.DateTimeFormat("en-AU", { weekday: "long", day: "numeric", month: "long", timeZone: "UTC" });
    return html`<header>
        <yen-icon-button icon="chevron-left" label="The month before" @click=${() => this.go(-1)}></yen-icon-button>
        <h3 aria-live="polite">${title}</h3>
        <yen-icon-button icon="chevron-right" label="The month after" @click=${() => this.go(1)}></yen-icon-button>
      </header>
      <table role="grid" aria-label=${title}>
        <thead>
          <tr>
            ${WEEKDAYS.map(([short, full]) => html`<th scope="col"><abbr title=${full}>${short}</abbr></th>`)}
          </tr>
        </thead>
        <tbody>
          ${weeks.map(
            (week) => html`<tr>
              ${week.map((d) => {
                const p = parts(d.date);
                const cls = ["day", d.inMonth ? "" : "outside", d.date === now ? "today" : "", marked.has(d.date) ? "marked" : ""]
                  .join(" ")
                  .trim();
                return html`<td>
                  <button
                    class=${cls}
                    type="button"
                    data-date=${d.date}
                    tabindex=${d.date === tabbable ? "0" : "-1"}
                    aria-pressed=${d.date === this.value ? "true" : "false"}
                    aria-label=${`${long.format(Date.UTC(p.year, p.month - 1, p.day))}${marked.has(d.date) ? ", something on" : ""}`}
                    aria-current=${d.date === now ? "date" : "false"}
                    @click=${() => this.choose(d.date)}
                    @keydown=${(e) => this.keyed(e, d.date)}
                  >
                    ${d.day}
                  </button>
                </td>`;
              })}
            </tr>`,
          )}
        </tbody>
      </table>`;
  }
}

customElements.define("yen-calendar", YenCalendar);
