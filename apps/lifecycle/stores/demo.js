// The demo: whether this visitor is trying the app without signing in, and
// the made-up people they start with, added once per device so deleting
// them does not bring them back. Every name is invented.
import { app } from "/_/yen.js";
import { people } from "./people.js";
import { tasks } from "./tasks.js";
import { notes } from "./notes.js";
import { matters } from "./matters.js";
import { addChecklist } from "./journey.js";
import { addDays, today } from "./days.js";

const SEEDED = "lifecycle-demo-seeded";
let started = false;

/** Whether this visitor is trying the demo. */
export function isDemo() {
  return !!app.demo;
}

function until(ready) {
  return new Promise((resolve) => {
    const check = () => (ready() ? resolve() : setTimeout(check, 100));
    check();
  });
}

function seededBefore() {
  try {
    return localStorage.getItem(SEEDED) === "1";
  } catch {
    return false;
  }
}

function markSeeded() {
  try {
    localStorage.setItem(SEEDED, "1");
  } catch {
    // Nothing to remember it in: the people are added again next time only if the list is empty.
  }
}

/** Add the sample people, once, for a demo visitor whose list is empty. */
export async function seedDemo() {
  if (!isDemo() || started || seededBefore()) return;
  started = true;
  await until(() => !people.loading.value && !tasks.loading.value && !notes.loading.value && !matters.loading.value);
  if (people.value.length) {
    markSeeded();
    return;
  }
  markSeeded();
  const now = today();
  const add = async (fields) => {
    const p = await people.add(fields);
    if (p) await addChecklist(p, p.stage);
    return p;
  };

  const priya = await add({
    name: "Priya Raman",
    role: "Marketing Coordinator",
    team: "Marketing",
    manager: "Tom Hale",
    location: "Sydney",
    employment: "full-time",
    stage: "onboarding",
    start_date: addDays(now, 7),
    probation_weeks: 26,
  });
  await add({
    name: "Liam Okafor",
    role: "Retail Assistant",
    team: "Retail",
    manager: "Grace Ng",
    location: "Melbourne",
    employment: "casual",
    stage: "onboarding",
    start_date: addDays(now, 14),
    probation_weeks: 12,
  });
  const mei = await add({
    name: "Mei Lin Zhou",
    role: "Finance Officer",
    team: "Finance",
    manager: "Daniel Brooks",
    location: "Sydney",
    employment: "part-time",
    stage: "active",
    start_date: addDays(now, -690),
  });
  const ahmed = await add({
    name: "Ahmed Hassan",
    role: "Customer Service Team Leader",
    team: "Customer Service",
    manager: "Sarah Kemp",
    location: "Brisbane",
    employment: "fixed-term",
    stage: "active",
    start_date: addDays(now, -400),
    contract_end: addDays(now, 60),
  });
  const jess = await add({
    name: "Jess Taylor",
    role: "Events Coordinator",
    team: "Events",
    manager: "Tom Hale",
    location: "Sydney",
    employment: "full-time",
    stage: "offboarding",
    start_date: addDays(now, -1100),
    last_day: addDays(now, 21),
  });

  if (priya) await notes.add({ person: priya.id, day: addDays(now, -2), kind: "File note", text: "Offer accepted. Contract sent for signing." });
  if (mei) {
    await notes.add({
      person: mei.id,
      day: addDays(now, -30),
      kind: "Check-in",
      text: "Asked about moving to four days a week from March. Asked her to put the request in writing. I will raise it with Daniel.",
    });
  }
  if (jess) {
    await notes.add({
      person: jess.id,
      day: addDays(now, -3),
      kind: "File note",
      text: "Resigned to take a role closer to home. Four weeks notice. Happy to help with the handover.",
    });
  }
  if (ahmed) {
    await notes.add({ person: ahmed.id, day: addDays(now, -90), kind: "Contract variation", text: "Contract extended by six months. Variation signed." });
    const matter = await matters.add({
      person: ahmed.id,
      title: "Complaint about rostering",
      kind: "Grievance",
      status: "open",
      opened: addDays(now, -10),
      summary: "Ahmed says the new roster gives him most of the late shifts. He thinks it is unfair. He has raised it twice with Sarah and wants the roster reviewed.",
    });
    if (matter) {
      await notes.add({
        person: ahmed.id,
        matter: matter.id,
        day: addDays(now, -8),
        kind: "Meeting",
        text: "Met with Ahmed. We went through the last eight weeks of rosters. He had most of the late shifts. I will raise it with Sarah and come back to him by Friday.",
      });
    }
  }
}
