// The ER matters, newest first: each person's, and those still open.
import { app, computed } from "/_/yen.js";
import { today } from "./days.js";

export const matters = app.collection("matters", { order: "-opened" });

export const MATTER_KINDS = [
  "Grievance",
  "Complaint",
  "Conduct",
  "Performance",
  "Psychosocial hazard",
  "Injury or illness",
  "Other",
];

export const STATUSES = [
  { value: "open", label: "Open" },
  { value: "on-hold", label: "On hold" },
  { value: "closed", label: "Closed" },
];

export function statusLabel(value) {
  return STATUSES.find((s) => s.value === (value ?? "open"))?.label ?? value;
}

export function mattersOf(personId) {
  return matters.value.filter((m) => m.person === personId);
}

export const openMatters = computed(() => matters.value.filter((m) => m.status !== "closed"));

export function openMatter(fields) {
  return matters.add({ status: "open", ...fields });
}

export function setKind(m, kind) {
  return matters.update(m.id, { kind });
}

export function setStatus(m, status) {
  return matters.update(m.id, { status, closed: status === "closed" ? today() : null });
}
