// Moving a person through their time with us: adding them with their
// checklist, moving stage, and removing them with everything about them.
import { people, stageLabel } from "./people.js";
import { tasks, tasksOf } from "./tasks.js";
import { notes, notesOf, notesOfMatter } from "./notes.js";
import { matters, mattersOf } from "./matters.js";
import { checklistFor } from "./checklists.js";
import { addYears, today } from "./days.js";

/** Add the standard checklist for `stage`, leaving out what is already on it. A yearly item is left out only while one is still open. */
export async function addChecklist(person, stage) {
  const all = new Set();
  const open = new Set();
  for (const t of tasksOf(person.id)) {
    all.add(t.title);
    if (!t.done) open.add(t.title);
  }
  const base = Date.now();
  for (const item of checklistFor(person, stage, today())) {
    if (item.repeat ? open.has(item.title) : all.has(item.title)) continue;
    await tasks.add({
      person: person.id,
      stage,
      title: item.title,
      due: item.due,
      done: false,
      position: base + item.position,
      ...(item.repeat ? { repeat: item.repeat } : {}),
    });
  }
}

/** Tick an item off, or put it back. Ticking off a yearly one books next year's. */
export async function tick(t) {
  await tasks.update(t.id, { done: !t.done });
  if (!t.done && t.repeat === "yearly") {
    const next = addYears(t.due ?? today(), 1);
    await tasks.add({ person: t.person, stage: t.stage, title: t.title, due: next, done: false, position: Date.now(), repeat: "yearly" });
  }
}

export async function addPerson(fields) {
  const made = await people.add(fields);
  if (made) await addChecklist(made, made.stage ?? "onboarding");
  return made;
}

export function savePerson(person, fields) {
  return people.update(person.id, fields);
}

export async function moveTo(person, stage) {
  if ((person.stage ?? "onboarding") === stage) return;
  const extra = stage === "left" && !person.last_day ? { last_day: today() } : {};
  await people.update(person.id, { stage, ...extra });
  await addChecklist({ ...person, ...extra, stage }, stage);
  await notes.add({ person: person.id, day: today(), kind: "Stage", text: `Moved to ${stageLabel(stage)}.` });
}

export async function removeMatter(m) {
  for (const n of notesOfMatter(m.id)) await notes.remove(n.id);
  await matters.remove(m.id);
}

export async function removePerson(person) {
  for (const t of tasksOf(person.id)) await tasks.remove(t.id);
  for (const n of notesOf(person.id)) await notes.remove(n.id);
  for (const m of mattersOf(person.id)) await matters.remove(m.id);
  await people.remove(person.id);
}
