// The conversation: what the app tells the AI about how to answer, the
// messages so far, and asking — the answer arriving as it is written. The
// conversation lasts while the page is open; the model remembers nothing
// between calls, so the whole of it is sent each time.
import { app, signal } from "/_/yen.js";

/** How the AI answers in this app: change it to change the helper. */
export const SYSTEM =
  "You help children aged 8 to 14 with their homework. Explain step by step in short, " +
  "friendly sentences, and ask a question back so they work out the answer themselves " +
  "rather than giving it to them. Never write their homework for them.";

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
      { system: SYSTEM, messages: asked.map((m) => ({ role: m.role, content: m.text })) },
      {
        onDelta: (piece) => {
          text += piece;
          messages.value = [...asked, { role: "assistant", text, pending: true }];
        },
      },
    );
    messages.value = [...asked, { role: "assistant", text }];
  } catch (e) {
    messages.value = [...asked, { role: "assistant", text: e.detail ?? "Sorry — I couldn't answer just now. Try again in a moment." }];
  } finally {
    busy.value = false;
  }
}

/** Start again. */
export function clear() {
  messages.value = [];
}
