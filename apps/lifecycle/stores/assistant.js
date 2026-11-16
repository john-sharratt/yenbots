// The assistant: how it answers, what it is told about Yen's people, and the
// conversation. The conversation lasts while the app is open; the model
// remembers nothing between calls, so the whole of it is sent each time.
import { app, signal } from "/_/yen.js";
import { employmentLabel, people, probationEnd, stageLabel } from "./people.js";
import { tasks } from "./tasks.js";
import { notes } from "./notes.js";
import { matters, statusLabel } from "./matters.js";
import { showDay, today } from "./days.js";

/** How the assistant answers: change it to change how it talks. */
export const RULES = `You are the assistant inside Yen's Employee Lifecycle app. Yen is a senior HR practitioner in Australia. She specialises in employee relations, industrial relations and WHS. The records below are hers and private to her.

You know Australian employment practice well: the Fair Work Act 2009 and the National Employment Standards, modern awards and enterprise agreements, unfair dismissal and general protections, WHS law including the duty to manage psychosocial hazards, and anti-discrimination law.

How you answer:
- Answer questions about a person from the records. If the records do not say, say so. Never make up a date, a fact or a quote.
- Write plain Australian English. Short sentences. No em dashes. No Oxford commas. Never a comma before "and". No jargon.
- When Yen asks for a draft (an email, a letter, a file note, an agenda) write it ready to use, in her voice, in the first person.
- Name legal risk plainly: unfair dismissal, general protections and adverse action, discrimination or a WHS duty. Say when a point turns on the award, the agreement or the contract. Say when it needs legal advice.
- Be direct. If the facts point away from Yen's view, say so and say why.
- Keep answers short unless Yen asks for more.`;

const LIMIT = 60000;

function line(parts) {
  return parts.filter(Boolean).join(" | ");
}

/** Everything the assistant is told, newest notes first, cut at the limit. */
export function context() {
  const names = new Map();
  for (const p of people.value) names.set(p.id, p.name);
  const titles = new Map();
  for (const m of matters.value) titles.set(m.id, m.title);
  const now = today();
  const out = [`Today is ${showDay(now)} (${now}).`, "", "PEOPLE"];
  for (const p of people.value) {
    out.push(
      "- " +
        line([
          p.name,
          p.role,
          p.team && `team ${p.team}`,
          p.manager && `manager ${p.manager}`,
          p.location,
          employmentLabel(p.employment),
          `stage ${stageLabel(p.stage)}`,
          p.start_date && `started ${p.start_date}`,
          p.probation_weeks && `probation ${p.probation_weeks} weeks`,
          probationEnd(p) && `probation ends ${probationEnd(p)}`,
          p.contract_end && `contract ends ${p.contract_end}`,
          p.last_day && `last day ${p.last_day}`,
        ]),
    );
    if (p.notes) out.push(`  About: ${p.notes}`);
  }
  out.push("", "OPEN CHECKLIST ITEMS");
  for (const t of tasks.value) if (!t.done) out.push("- " + line([names.get(t.person), t.title, t.due && `due ${t.due}`]));
  out.push("", "ER MATTERS");
  for (const m of matters.value) {
    out.push("- " + line([names.get(m.person), m.title, m.kind, statusLabel(m.status), m.opened && `opened ${m.opened}`, m.closed && `closed ${m.closed}`]));
    if (m.summary) out.push(`  Summary: ${m.summary}`);
    if (m.outcome) out.push(`  Outcome: ${m.outcome}`);
  }
  out.push("", "FILE NOTES, NEWEST FIRST");
  let text = out.join("\n");
  for (const n of notes.value) {
    const entry = "\n- " + line([n.day, names.get(n.person), n.kind, n.matter && `matter: ${titles.get(n.matter) ?? ""}`]) + `\n  ${n.text}`;
    if (text.length + entry.length > LIMIT) {
      text += "\n(Older notes left out.)";
      break;
    }
    text += entry;
  }
  return text;
}

/** `[{role, text, pending}]`, as `<yen-chat>` shows them. */
export const messages = signal([]);

/** Whether an answer is on its way. */
export const busy = signal(false);

/** Ask `words`; the answer is shown as it arrives. */
export async function ask(words) {
  const asked = [...messages.value, { role: "user", text: words }];
  messages.value = [...asked, { role: "assistant", text: "", pending: true }];
  busy.value = true;
  let text = "";
  try {
    await app.ai.chat(
      {
        system: `${RULES}\n\nYEN'S RECORDS\n${context()}`,
        messages: asked.map((m) => ({ role: m.role, content: m.text })),
        max_tokens: 2000,
      },
      {
        onDelta: (piece) => {
          text += piece;
          messages.value = [...asked, { role: "assistant", text, pending: true }];
        },
      },
    );
    messages.value = [...asked, { role: "assistant", text }];
  } catch (e) {
    messages.value = [...asked, { role: "assistant", text: e.detail ?? "I couldn't answer just now. Try again in a moment." }];
  } finally {
    busy.value = false;
  }
}

/** Start again. */
export function clear() {
  messages.value = [];
}
