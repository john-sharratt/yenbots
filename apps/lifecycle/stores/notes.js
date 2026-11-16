// The file notes, newest first: each person's, and each ER matter's.
import { app } from "/_/yen.js";

export const notes = app.collection("notes", { order: "-day" });

export const KINDS = [
  "File note",
  "Meeting",
  "Check-in",
  "Contract variation",
  "Review",
  "Pay",
  "Flexible work",
  "Leave",
  "Performance",
  "Other",
];

export function notesOf(personId) {
  return notes.value.filter((n) => n.person === personId);
}

export function notesOfMatter(matterId) {
  return notes.value.filter((n) => n.matter === matterId);
}

export function addNote(fields) {
  return notes.add(fields);
}

export function removeNote(n) {
  return notes.remove(n.id);
}
