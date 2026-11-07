// Each person's reminders, the soonest first; those still to come and
// those gone by.
import { app, computed } from "/_/yen.js";

export const reminders = app.collection("reminders", { order: "due_ms" });

export const coming = computed(() => reminders.value.filter((r) => r.due_ms > Date.now()));
export const past = computed(() => reminders.value.filter((r) => r.due_ms <= Date.now()).reverse());

/** `ms` as people say it: "Sunday 8 November, 7:30 pm". */
export function when(ms) {
  return new Date(ms).toLocaleString("en-AU", {
    weekday: "long",
    day: "numeric",
    month: "long",
    hour: "numeric",
    minute: "2-digit",
  });
}
