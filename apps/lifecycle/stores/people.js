// The people, by name: where each is in their time with us, and how they are employed.
import { app, computed } from "/_/yen.js";
import { addDays } from "./days.js";

export const people = app.collection("people", { order: "name" });

export const STAGES = [
  { value: "onboarding", label: "Onboarding" },
  { value: "active", label: "Employed" },
  { value: "offboarding", label: "Leaving" },
  { value: "left", label: "Left" },
];

export const EMPLOYMENT = [
  { value: "full-time", label: "Permanent full time" },
  { value: "part-time", label: "Permanent part time" },
  { value: "fixed-term", label: "Fixed term" },
  { value: "casual", label: "Casual" },
  { value: "contractor", label: "Contractor" },
];

export function stageLabel(value) {
  return STAGES.find((s) => s.value === (value ?? "onboarding"))?.label ?? value;
}

export function employmentLabel(value) {
  return EMPLOYMENT.find((e) => e.value === value)?.label ?? value ?? "";
}

/** The people at `stage`. */
export function inStage(stage) {
  return people.value.filter((p) => (p.stage ?? "onboarding") === stage);
}

/** How many people are at each stage. */
export const counts = computed(() => {
  const c = {};
  for (const p of people.value) {
    const s = p.stage ?? "onboarding";
    c[s] = (c[s] ?? 0) + 1;
  }
  return c;
});

/** The day probation ends: the start date plus its weeks. Empty when either is missing. */
export function probationEnd(p) {
  if (p.start_date && p.probation_weeks) return addDays(p.start_date, Math.round(p.probation_weeks * 7));
  return p.probation_end ?? "";
}

export function nameOf(id) {
  return people.get(id)?.name ?? "";
}
