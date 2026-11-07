// The list: everyone's, in the order it was put in. An item moved trades
// places with its neighbour; a new one goes to the end.
import { app, computed } from "/_/yen.js";

export const items = app.collection("items", { order: "position" });

/** What is still to get, then what is ticked off. */
export const ordered = computed(() => [
  ...items.value.filter((i) => !i.done),
  ...items.value.filter((i) => i.done),
]);

/** Add `text` at the end of the list. */
export function add(text) {
  const last = items.value.reduce((most, i) => Math.max(most, i.position ?? 0), 0);
  return items.add({ text, done: false, position: last + 1 });
}

/** Tick `item` off, or put it back. */
export function toggle(item) {
  return items.update(item.id, { done: !item.done });
}

/** Move `item` one place up (-1) or down (+1) among those still to get. */
export async function move(item, by) {
  const open = items.value.filter((i) => !i.done);
  const at = open.findIndex((i) => i.id === item.id);
  const other = open[at + by];
  if (at < 0 || !other) return;
  await items.update(item.id, { position: other.position });
  await items.update(other.id, { position: item.position });
}
