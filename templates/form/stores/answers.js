// The answers, and what they come to: how many chose each day, the
// average rating, and who is bringing what.
import { app, computed } from "/_/yen.js";

/** The days to choose from: change them to change the question. */
export const DAYS = ["Saturday 14th", "Sunday 15th", "Saturday 21st"];

export const answers = app.collection("answers", { order: "-created_ms" });

/** How many chose each day, as the chart draws it — every day, chosen or not. */
export const byDay = computed(() =>
  DAYS.map((day) => ({ label: day, value: answers.value.filter((a) => a.day === day).length })),
);

/** The average rating, to one place; null before the first answer. */
export const rating = computed(() => {
  const all = answers.value;
  if (all.length === 0) return null;
  return Math.round((all.reduce((sum, a) => sum + a.rating, 0) / all.length) * 10) / 10;
});

/** Who is bringing something, and what. */
export const bringing = computed(() => answers.value.filter((a) => a.bringing));
