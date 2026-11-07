# AI, pictures and files

An app can chat with the model, draw pictures, answer questions about its own
data and keep files (`candle/docs/yenbot_app_host.md` §14, §6.1). The model
and the image engine are Zen's, reached by the site itself — no page or
workflow ever holds a credential for them.

An app that uses the AI says so in its `app.yaml`:

```yaml
ai: true
```

Without it every call is refused: `this app does not use the AI: add
\`ai: true\` to its app.yaml`. There is no budget: an app chooses its prompts
and sizes, and every call is kept — which app, which account, what was asked,
how long it took, how many tokens — for the admin page.

## A chat

```js
// a page
const messages = [{ role: "user", content: "Something with chicken tonight?" }];
const { content } = await app.ai.chat(
  { system: "You suggest one dinner, in a sentence, for a family of four.", messages },
  { onDelta: (piece) => (this.answer += piece) },   // the answer as it is written
);
```

- `system` is the app's own instructions; `messages` is the conversation,
  `{role: "user" | "assistant", content}`, the person's last. Send the whole
  conversation each time: the model remembers nothing between calls.
- `max_tokens` caps the answer, up to 4000 (1000 when absent).
- Leave out `onDelta` to get the answer whole.
- `<yen-chat>` is the panel for a conversation (`design.md`): the page keeps
  the messages and sends them; the panel shows them, the answer as it arrives.

**An answer of a shape**: give `schema`, a JSON Schema, and the answer comes
as JSON, in `json`, held to the schema's `type`, `properties`, `required`,
`items` and `enum` — asked again once if it does not fit:

```js
const { json } = await app.ai.chat({
  system: "Plan a week of dinners.",
  messages: [{ role: "user", content: "Mostly vegetarian, one fish night." }],
  schema: { type: "object", required: ["dinners"], properties: {
    dinners: { type: "array", items: { type: "object", required: ["day", "dish"],
      properties: { day: { type: "string" }, dish: { type: "string" } } } } } },
});
for (const d of json.dinners) await app.collection("dinners").add(d);
```

## A picture

```js
const { file, seed } = await app.ai.image(
  { prompt: "A bowl of phở, soft flat vector, cream ground", width: 768, height: 512 },
  { onProgress: (p) => (this.progress = p.total ? p.done / p.total : 0) },
);
await app.collection("recipes").update(recipe.id, { photo: file.id });
```

- Sides from 64 to 1024, each a multiple of 16; 1024 square when absent.
- A picture takes half a minute to a few minutes: show `<yen-progress>` from
  `onProgress`, and tell the person it is coming.
- It is kept as one of the app's files (below); its `id` goes in a `file`
  field. `seed` draws the same again.
- **To change a picture** — brighter, a hat on the cat — give `from`, the
  picture's file id, and say what it should become; `hold` (0 to 0.95, 0.5
  when absent) is how closely it keeps to the original.
- Many pictures — forty for a storybook — go in a workflow (`ctx.ai.image`,
  `workflows.md`) with `deadline: 10m`, run from a button
  (`app.workflows.run`), not one page call after another.
- The model draws; it does not look. An app cannot ask about a photo.

## A question about the app's data

```js
const { answer, ids } = await app.ai.ask("Which of my recipes use chicken?", { from: "recipes" });
```

The collection's `search:` fields are searched for the question's words, as
the person may see them; the best dozen records go to the model with their
ids, and it answers from them alone, with the ids it drew on — link to them
(`r/${id}`). A collection with no `search:` fields cannot be asked.

## Files

```js
const { id, url, thumb } = await app.files.upload(input.files[0]);   // a photo, a PDF
html`<img src=${app.files.url(id, { thumb: true })} alt=${recipe.title}>`;
```

A picture (PNG, JPEG, WebP, GIF) or a PDF, up to 20 MB, told by its bytes;
a picture gets a 480 thumbnail. A record holds the id in a field of `type:
file`. Those who may open the app may see its files. `<yen-image-picker>`
takes a photo or chooses one (`design.md`).

## From a workflow

`ctx.ai.chat(ask)`, `ctx.ai.image(draw)` and `ctx.ai.ask(from, question)` do
the same, answered whole, as the run's account or the app (`workflows.md`).

## When it goes wrong

A refusal throws an `ApiError` whose `code` is `refused` and whose `detail`
says why, in words to show: the app lacks `ai: true`, a size is out of range,
the image engine declined a prompt, the model was busy. Every call is in the
app's AI usage on the admin page.
