// The cards, by title; the tags they carry; search over their words.
import { app, computed } from "/_/yen.js";

export const items = app.collection("items", { order: "title" });

/** Every tag any card carries, in order. */
export const tags = computed(() => [...new Set(items.value.flatMap((i) => i.tags ?? []))].sort());

/** The cards carrying `tag`, or every card. */
export function tagged(tag) {
  return tag ? items.value.filter((i) => (i.tags ?? []).includes(tag)) : items.value;
}

/** The cards whose words match `words`, best first. */
export function find(words) {
  return items.search(words, 50);
}

/** Make a card, keeping `photo` — a `File` from the picker — first. */
export async function make({ title, notes, tags, photo }) {
  const kept = photo ? await app.files.upload(photo) : null;
  return items.add({ title, notes, tags, ...(kept ? { photo: kept.id } : {}) });
}

/** Where card `i`'s photo is shown from. */
export function photoOf(i, { thumb = false } = {}) {
  return i.photo ? app.files.url(i.photo, { thumb }) : null;
}
