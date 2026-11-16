// What is coming up: key dates in the next month and checklist items due in
// the next fortnight or already late, soonest first.
import { computed } from "/_/yen.js";
import { people } from "./people.js";
import { tasks } from "./tasks.js";
import { addDays, daysFrom, fromNow, today } from "./days.js";

const DATES = [
  ["start_date", "Starts"],
  ["probation_end", "Probation ends"],
  ["contract_end", "Contract ends"],
  ["last_day", "Last day"],
];
const AHEAD = 30;
const TASKS_AHEAD = 14;

export const upcoming = computed(() => {
  const now = today();
  const until = addDays(now, AHEAD);
  const out = [];
  const byId = new Map();
  for (const p of people.value) {
    byId.set(p.id, p);
    if (p.stage === "left") continue;
    for (const [field, label] of DATES) {
      const day = p[field];
      if (day && day >= now && day <= until) out.push({ id: `${p.id}-${field}`, day, label, who: p.name, href: `p/${p.id}`, late: false });
    }
  }
  const taskUntil = addDays(now, TASKS_AHEAD);
  for (const t of tasks.value) {
    if (t.done || !t.due || t.due > taskUntil) continue;
    const p = byId.get(t.person);
    if (!p || p.stage === "left") continue;
    out.push({ id: t.id, day: t.due, label: t.title, who: p.name, href: `p/${p.id}`, late: t.due < now });
  }
  return out.sort((a, b) => a.day.localeCompare(b.day));
});

export const lateCount = computed(() => upcoming.value.filter((u) => u.late).length);

/** A word on the person's row when a key date is close: `Probation ends in 9 days`. */
export function flagOf(p) {
  const now = today();
  for (const [field, label] of DATES.slice(1)) {
    const day = p[field];
    if (day && day >= now && daysFrom(now, day) <= AHEAD) return `${label} ${fromNow(day)}`;
  }
  return "";
}
