// `<yen-date-picker label="Due" type="datetime" .value=${…}>` (§8.5): a day,
// a time of day, or both, with the device's own picker and the device's
// time zone. The value is what the field it fills holds (§7.5):
//
// | `type` | value |
// |---|---|
// | `date` (the default) | `YYYY-MM-DD` |
// | `time` | `HH:MM` |
// | `datetime` | milliseconds since the epoch, a `time` field's value |
//
// A date picker offers "Today" beside it, as one tap.

import { css, html, live, nothing } from "../../vendor/lit-all.min.js";
import { YenField } from "../field-element.js";
import { isoDate } from "../month.js";
import { base, button, control, field } from "../styles.js";
import "./icon.js";

const pad = (n) => String(n).padStart(2, "0");

/** Milliseconds as a `datetime-local` control shows them, in this device's zone. */
function toLocal(ms) {
  const d = new Date(ms);
  return `${isoDate(d.getFullYear(), d.getMonth() + 1, d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

const INPUT = { date: "date", time: "time", datetime: "datetime-local" };
const ICON = { date: "calendar", time: "clock", datetime: "calendar" };

export class YenDatePicker extends YenField {
  static properties = {
    ...YenField.properties,
    type: { type: String },
    min: { type: String },
    max: { type: String },
  };

  static styles = [
    base,
    control,
    field,
    button,
    css`
      :host {
        display: block;
      }
      .row {
        display: flex;
        gap: var(--yen-space-2);
        align-items: stretch;
      }
      .wrap {
        position: relative;
        flex: 1;
      }
      .wrap yen-icon {
        position: absolute;
        left: var(--yen-space-3);
        top: 50%;
        transform: translateY(-50%);
        color: var(--yen-ink-2);
        pointer-events: none;
      }
      .control {
        padding-left: calc(var(--yen-space-3) * 2 + 1.25rem);
        font-variant-numeric: tabular-nums;
      }
      .control::-webkit-calendar-picker-indicator {
        opacity: 0.6;
        cursor: pointer;
      }
    `,
  ];

  constructor() {
    super();
    this.type = "date";
    this.value = "";
  }

  /** The value as the control shows it. */
  shown() {
    if (this.value === undefined || this.value === null || this.value === "") return "";
    return this.type === "datetime" ? toLocal(Number(this.value)) : String(this.value);
  }

  /** What the control shows, as the value. */
  read(text) {
    if (text === "") return "";
    return this.type === "datetime" ? new Date(text).getTime() : text;
  }

  today() {
    const d = new Date();
    this.take(isoDate(d.getFullYear(), d.getMonth() + 1, d.getDate()), true);
  }

  render() {
    const kind = INPUT[this.type] ? this.type : "date";
    return html`<div class="field">
      ${this.labelPart()}
      <div class="row">
        <div class="wrap">
          <yen-icon name=${ICON[kind]}></yen-icon>
          <input
            class="control"
            id=${this.ids.control}
            type=${INPUT[kind]}
            .value=${live(this.shown())}
            min=${this.min ?? nothing}
            max=${this.max ?? nothing}
            ?required=${this.required}
            ?disabled=${this.disabled}
            aria-invalid=${this.error ? "true" : "false"}
            aria-describedby=${this.describedBy()}
            @input=${(e) => {
              e.stopPropagation();
              this.take(this.read(e.target.value), false);
            }}
            @change=${(e) => this.take(this.read(e.target.value), true)}
          />
        </div>
        ${kind === "date" && !this.disabled
          ? html`<button class="btn secondary" type="button" @click=${this.today}>Today</button>`
          : nothing}
      </div>
      ${this.notes()}
    </div>`;
  }
}

customElements.define("yen-date-picker", YenDatePicker);
