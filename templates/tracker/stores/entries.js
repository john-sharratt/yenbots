// The numbers logged, the oldest day first; the chart's rows; the latest.
import { app, computed } from "/_/yen.js";

export const entries = app.collection("entries", { order: "day" });

/** The newest first, for the list. */
export const newest = computed(() => [...entries.value].reverse());

/** The last thirty, as the chart draws them. */
export const rows = computed(() =>
  entries.value.slice(-30).map((e) => ({ label: short(e.day), value: e.value })),
);

/** The latest number, or null before the first. */
export const latest = computed(() => entries.value.at(-1)?.value ?? null);

/** `day` as a chart's label: "7 Nov". */
export function short(day) {
  const [y, m, d] = day.split("-").map(Number);
  return new Date(y, m - 1, d).toLocaleDateString("en-AU", { day: "numeric", month: "short" });
}

/** A number as money: "$1,250". */
export function money(n) {
  return n.toLocaleString("en-AU", { style: "currency", currency: "AUD", maximumFractionDigits: 0 });
}

/** Today on this device, `YYYY-MM-DD`. */
export function today() {
  const d = new Date();
  const two = (n) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${two(d.getMonth() + 1)}-${two(d.getDate())}`;
}
