import { eventsOf, Ai } from "/_/framework/ai.js";
import { Api } from "/_/framework/api.js";
import { Files } from "/_/framework/files.js";
import { fakeFetch } from "./fakes.js";
import { eq, test } from "./harness.js";

// The engine these tests run in has no `TextDecoder`; the test's answers
// are ASCII, which this reads as a browser's would.
globalThis.TextDecoder ??= class {
  decode(bytes) {
    return bytes ? String.fromCharCode(...bytes) : "";
  }
};

const bytes = (s) => Uint8Array.from(s, (c) => c.charCodeAt(0));

/** A `fetch` answering with a stream of `chunks`, as the site's events arrive. */
function streaming(chunks) {
  const calls = [];
  const fetch = async (url, init) => {
    calls.push({ url, ...init, body: JSON.parse(init.body) });
    let at = 0;
    return {
      ok: true,
      status: 200,
      body: {
        getReader: () => ({
          read: async () => (at < chunks.length ? { value: bytes(chunks[at++]), done: false } : { value: undefined, done: true }),
        }),
      },
    };
  };
  return { fetch, calls };
}

test("server-sent events are read whole, what is still arriving kept", () => {
  const { events, rest } = eventsOf('event: delta\ndata: {"content":"Ph"}\n\nevent: done\ndata: {"con');
  eq(events, [{ event: "delta", data: { content: "Ph" } }]);
  eq(rest, 'event: done\ndata: {"con');
  eq(eventsOf(": keep-alive\n\ndata: {}\n\n").events, [{ event: "message", data: {} }]);
});

test("a chat's answer arrives as it is written, however the stream is cut", async () => {
  const f = streaming(['event: delta\ndata: {"content":"Ph', 'o "}\n\nevent: delta\ndata: {"content":"tonight."}\n\nevent: done\ndata: {"content":"Pho tonight.","json":null}\n\n']);
  const ai = new Ai(new Api("meals", f.fetch));
  const pieces = [];
  const done = await ai.chat({ messages: [{ role: "user", content: "Dinner?" }] }, { onDelta: (p) => pieces.push(p) });
  eq(pieces, ["Pho ", "tonight."]);
  eq(done, { content: "Pho tonight.", json: null });
  eq([f.calls[0].url, f.calls[0].headers["X-Yen-App"], f.calls[0].body.stream], ["/api/v1/ai/chat", "meals", true]);
});

test("a picture's progress is told, and a refusal mid-stream throws", async () => {
  const f = streaming(['event: progress\ndata: {"event":"step","done":3,"total":24}\n\nevent: error\ndata: {"error":"refused","detail":"no room"}\n\n']);
  const ai = new Ai(new Api("recipes", f.fetch));
  const seen = [];
  let thrown;
  try {
    await ai.image({ prompt: "A bowl" }, { onProgress: (p) => seen.push(p.done) });
  } catch (e) {
    thrown = e;
  }
  eq(seen, [3]);
  eq([thrown.code, thrown.detail], ["refused", "no room"]);
});

test("a question, and a chat held to a schema, are answered whole", async () => {
  const f = fakeFetch([
    [200, { answer: "Two.", ids: ["r1"], read: 3 }],
    [200, { content: '{"dish":"Pho"}', json: { dish: "Pho" } }],
  ]);
  const ai = new Ai(new Api("recipes", f.fetch));
  eq(await ai.ask("Which use chicken?", { from: "recipes" }), { answer: "Two.", ids: ["r1"], read: 3 });
  const schema = { type: "object" };
  eq((await ai.chat({ messages: [], schema }, { onDelta: () => {} })).json, { dish: "Pho" });
  eq(
    f.calls.map((c) => [c.url, c.body]),
    [
      ["/api/v1/ai/ask", { question: "Which use chicken?", from: "recipes" }],
      ["/api/v1/ai/chat", { messages: [], schema }],
    ],
  );
});

test("a file is uploaded as its bytes, named, and its addresses made", async () => {
  const calls = [];
  const fetch = async (url, init) => {
    calls.push({ url, init });
    return { ok: true, status: 201, text: async () => JSON.stringify({ id: "f1", url: "/api/v1/files/recipes/f1" }) };
  };
  const files = new Files(new Api("recipes", fetch));
  const blob = { name: "phở.jpg", type: "image/jpeg" };
  eq((await files.upload(blob)).id, "f1");
  eq(calls[0].url, "/api/v1/files/recipes?name=ph%E1%BB%9F.jpg");
  eq([calls[0].init.headers["Content-Type"], calls[0].init.body === blob], ["image/jpeg", true]);
  eq(files.url("f1", { thumb: true }), "/api/v1/files/recipes/f1/thumb");
});
