// `app.ai` (§14): the model and the image engine, through the site — for an
// app whose `app.yaml` says `ai: true`.
//
//   const { content } = await app.ai.chat({ system, messages }, { onDelta: (piece) => … });
//   const { file, seed } = await app.ai.image({ prompt, width, height }, { onProgress: (p) => … });
//   const { answer, ids } = await app.ai.ask("Which recipes use chicken?", { from: "recipes" });
//
// With `onDelta` a chat's answer arrives as it is written, and with
// `onProgress` a picture's progress as it is drawn: both are read from the
// site's server-sent events. A refusal throws an `ApiError`, as any call.

import { ApiError, nfc, Offline } from "./api.js";

/**
 * The server-sent events whole in `text`, and what is left of it — the
 * start of an event still arriving. An event is `{event, data}`, its data
 * read as JSON.
 */
export function eventsOf(text) {
  const events = [];
  const blocks = text.split(/\r?\n\r?\n/);
  const rest = blocks.pop();
  for (const block of blocks) {
    let event = "message";
    const data = [];
    for (const line of block.split(/\r?\n/)) {
      if (line.startsWith("event:")) event = line.slice(6).trim();
      else if (line.startsWith("data:")) data.push(line.slice(5).replace(/^ /, ""));
    }
    if (!data.length) continue;
    let parsed = null;
    try {
      parsed = JSON.parse(data.join("\n"));
    } catch {
      continue;
    }
    events.push({ event, data: parsed });
  }
  return { events, rest };
}

export class Ai {
  /** @param api the app's `Api` */
  constructor(api) {
    this.api = api;
  }

  /**
   * `POST /api/v1/ai/<what>` streamed: each event but the last handed to
   * `each`; answers the last, `done` — or throws the `error` it was.
   */
  async streamed(what, body, each) {
    let res;
    try {
      res = await this.api.fetch(`/api/v1/ai/${what}`, {
        method: "POST",
        credentials: "same-origin",
        headers: { "X-Yen-App": this.api.app, "Content-Type": "application/json", Accept: "text/event-stream" },
        body: JSON.stringify(nfc({ ...body, stream: true })),
      });
    } catch (e) {
      throw new Offline(e);
    }
    if (!res.ok) {
      const text = await res.text();
      let json = null;
      try {
        json = JSON.parse(text);
      } catch {
        json = { error: `http_${res.status}`, detail: text.slice(0, 200) };
      }
      throw new ApiError(res.status, json);
    }
    const reader = res.body.getReader();
    const decoder = new TextDecoder();
    let held = "";
    for (;;) {
      const { value, done } = await reader.read();
      if (value) held += decoder.decode(value, { stream: true });
      const { events, rest } = eventsOf(done ? `${held}\n\n` : held);
      held = rest;
      for (const e of events) {
        if (e.event === "done") return e.data;
        if (e.event === "error") throw new ApiError(422, e.data);
        each(e);
      }
      if (done) throw new Offline(new Error("the answer stopped before it ended"));
    }
  }

  /** A chat: `{messages, system, max_tokens, schema}`; answers `{content, json}`. */
  async chat(ask, { onDelta } = {}) {
    if (!onDelta || ask.schema) return this.api.call("POST", "/ai/chat", ask);
    return this.streamed("chat", ask, (e) => {
      if (e.event === "delta") onDelta(e.data.content);
    });
  }

  /** A picture drawn, or one changed (`from`); answers `{file, seed}`. */
  async image(draw, { onProgress } = {}) {
    if (!onProgress) return this.api.call("POST", "/ai/image", draw);
    return this.streamed("image", draw, (e) => {
      if (e.event === "progress") onProgress(e.data);
    });
  }

  /** A question about collection `from`, answered from its records: `{answer, ids, read}`. */
  async ask(question, { from }) {
    return this.api.call("POST", "/ai/ask", { question, from });
  }
}
