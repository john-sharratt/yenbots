// What is on, by day; the days of a week; the days of a month with
// something on.
import { app, computed } from "/_/yen.js";

export const plans = app.collection("plans", { order: "day" });

const two = (n) => String(n).padStart(2, "0");

/** `d` (a `Date`) as `YYYY-MM-DD`. */
export function iso(d) {
  return `${d.getFullYear()}-${two(d.getMonth() + 1)}-${two(d.getDate())}`;
}

/** Today on this device, `YYYY-MM-DD`. */
export function today() {
  return iso(new Date());
}

/** The seven days, Monday first, of the week holding `day`. */
export function weekOf(day) {
  const [y, m, d] = day.split("-").map(Number);
  const at = new Date(y, m - 1, d);
  const monday = new Date(y, m - 1, d - ((at.getDay() + 6) % 7));
  return Array.from({ length: 7 }, (_, i) => iso(new Date(monday.getFullYear(), monday.getMonth(), monday.getDate() + i)));
}

/** `day` moved by `days`. */
export function shifted(day, days) {
  const [y, m, d] = day.split("-").map(Number);
  return iso(new Date(y, m - 1, d + days));
}

/** What is on `day`, by time — those with none first. */
export function on(day) {
  return plans.value.filter((p) => p.day === day).sort((a, b) => (a.time ?? "").localeCompare(b.time ?? ""));
}

/** Every day with something on, for the month's dots. */
export const marked = computed(() => [...new Set(plans.value.map((p) => p.day))]);

/** `day` as people say it: "Monday 9 November". */
export function spoken(day) {
  const [y, m, d] = day.split("-").map(Number);
  return new Date(y, m - 1, d).toLocaleDateString("en-AU", { weekday: "long", day: "numeric", month: "long" });
}
