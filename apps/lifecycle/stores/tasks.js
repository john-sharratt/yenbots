// The checklist items: each person's, open ones first, soonest due first.
import { app } from "/_/yen.js";

export const tasks = app.collection("tasks", { order: "position" });

export function tasksOf(personId) {
  return tasks.value
    .filter((t) => t.person === personId)
    .sort((a, b) => Number(!!a.done) - Number(!!b.done) || (a.due ?? "9999").localeCompare(b.due ?? "9999") || (a.position ?? 0) - (b.position ?? 0));
}

export function progressOf(personId) {
  const all = tasksOf(personId);
  return { done: all.filter((t) => t.done).length, total: all.length };
}

export function toggle(t) {
  return tasks.update(t.id, { done: !t.done });
}

export function removeTask(t) {
  return tasks.remove(t.id);
}

export function addTask(person, stage, title, due) {
  return tasks.add({ person, stage, title, done: false, position: Date.now(), ...(due ? { due } : {}) });
}
