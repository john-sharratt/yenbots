// The standard checklists, added when a person starts onboarding or leaving.
// `from` is the day each item is counted from: now, the start date, the end
// of probation or the last day. `days` is how far from it the item is due.
// `only` keeps an item to some kinds of employment.
import { addDays } from "./days.js";
import { probationEnd } from "./people.js";

const EMPLOYEE = ["full-time", "part-time", "fixed-term", "casual"];

export const CHECKLISTS = {
  onboarding: [
    { title: "Offer letter and contract signed", from: "start", days: -7, only: EMPLOYEE },
    { title: "Contractor agreement signed", from: "start", days: -7, only: ["contractor"] },
    { title: "ABN and insurance certificates checked", from: "start", days: -7, only: ["contractor"] },
    { title: "Right to work checked (VEVO if on a visa)", from: "start", days: -3 },
    { title: "Payroll and HRIS set up", from: "start", days: -3 },
    { title: "IT account and equipment ready", from: "start", days: -3 },
    { title: "First week plan agreed with manager", from: "start", days: -3 },
    { title: "Building access arranged", from: "start", days: -1 },
    { title: "Fair Work Information Statement given", from: "start", days: -1, only: EMPLOYEE },
    { title: "Casual Employment Information Statement given", from: "start", days: -1, only: ["casual"] },
    { title: "Fixed Term Contract Information Statement given", from: "start", days: -1, only: ["fixed-term"] },
    { title: "TFN declaration received", from: "start", days: 0, only: EMPLOYEE },
    { title: "Super choice form received or stapled fund requested", from: "start", days: 0, only: EMPLOYEE },
    { title: "Bank details and emergency contact received", from: "start", days: 0 },
    { title: "WHS induction done", from: "start", days: 1 },
    { title: "Code of conduct and key policies acknowledged", from: "start", days: 7 },
    { title: "One month check-in", from: "start", days: 30, only: EMPLOYEE },
    { title: "Three month check-in", from: "start", days: 90, only: EMPLOYEE },
    { title: "Probation review meeting", from: "probation", days: -14, only: EMPLOYEE },
    { title: "Probation outcome confirmed in writing", from: "probation", days: -7, only: EMPLOYEE },
  ],
  offboarding: [
    { title: "Resignation received or termination letter issued", from: "now", days: 0 },
    { title: "Last day and notice confirmed in writing", from: "now", days: 2 },
    { title: "Handover plan agreed with manager", from: "last", days: -14 },
    { title: "Exit interview booked", from: "last", days: -7 },
    { title: "Final pay worked out: leave, notice and other entitlements", from: "last", days: -5, only: EMPLOYEE },
    { title: "Equipment returned", from: "last", days: 0 },
    { title: "System, email and building access removed", from: "last", days: 0 },
    { title: "Payroll and HRIS updated", from: "last", days: 0 },
    { title: "Final pay paid on time (check the award or agreement)", from: "last", days: 7, only: EMPLOYEE },
    { title: "Final invoice received", from: "last", days: 7, only: ["contractor"] },
    { title: "Separation certificate if asked for", from: "last", days: 14, only: EMPLOYEE },
    { title: "Statement of service if asked for", from: "last", days: 14, only: EMPLOYEE },
  ],
};

/** The checklist `person` gets on reaching `stage`, each item with its due day. */
export function checklistFor(person, stage, now) {
  const kind = person.employment ?? "full-time";
  const start = person.start_date ?? now;
  const from = {
    now,
    start,
    probation: probationEnd(person),
    last: person.last_day ?? addDays(now, 28),
  };
  const items = CHECKLISTS[stage] ?? [];
  return items
    .filter((i) => !i.only || i.only.includes(kind))
    .filter((i) => i.from !== "probation" || from.probation)
    .map((i, n) => ({ title: i.title, due: addDays(from[i.from], i.days), position: n }));
}
