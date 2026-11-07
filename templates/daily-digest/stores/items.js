// What is on, the soonest first; and the morning note sent now.
import { app, computed } from "/_/yen.js";

export const items = app.collection("items", { order: "due_ms" });

export const coming = computed(() => items.value.filter((i) => i.due_ms > Date.now()));

/** Send this morning's note now, as the 07:00 job does; answers how many it named. */
export function sendNow() {
  return app.workflows.run("morning");
}

/** `ms` as people say it: "Sunday 8 November, 7:30 pm". */
export function when(ms) {
  return new Date(ms).toLocaleString("en-AU", { weekday: "long", day: "numeric", month: "long", hour: "numeric", minute: "2-digit" });
}
