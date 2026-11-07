// Each person's own entries, the newest day first.
import { app } from "/_/yen.js";

export const entries = app.collection("entries", { order: "-day" });

/** Today on this device, `YYYY-MM-DD`. */
export function today() {
  const d = new Date();
  const two = (n) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${two(d.getMonth() + 1)}-${two(d.getDate())}`;
}

/** `day` as people say it: "Saturday 7 November". */
export function spoken(day) {
  const [y, m, d] = day.split("-").map(Number);
  return new Date(y, m - 1, d).toLocaleDateString("en-AU", { weekday: "long", day: "numeric", month: "long" });
}

/** Write an entry, keeping `photo` — a `File` from the picker — first when there is one. */
export async function write({ day, text, photo }) {
  const kept = photo ? await app.files.upload(photo) : null;
  return entries.add({ day, text, ...(kept ? { photo: kept.id } : {}) });
}

/** Where entry `e`'s photo is shown from, small or whole. */
export function photoOf(e, { thumb = false } = {}) {
  return e.photo ? app.files.url(e.photo, { thumb }) : null;
}
