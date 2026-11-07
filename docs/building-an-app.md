# Building an app

Edition: 1

Read this whole guide before every new app, and before any change that adds
a page, a collection, a workflow or a schedule to one. Every app's
`app.yaml` says the edition it was built to (`guide: 1`); the gate refuses a
new app without it and a changed app built to an older edition, so when this
edition number rises, an app changed afterwards is brought up to it — which
means reading this again.

The design behind it is `candle/docs/yenbot_app_host.md`; this guide is what
a builder needs of it.

## Before anything

1. **Read the journal.** `journal_grep` the `yenbot` journal for the app's
   name, and read Yen's taste entries (`docs/journal.md`). What she decided
   last time binds this time.
2. **Understand what she wants, in her words.** Yen is not a coder: ask what
   she wants it to *do*, never how. One short question beats a wrong app.
   Don't show her code; show her the app.
3. **Find the template.** Almost every request is close to one (below).

## The templates

A new app starts as a copy of the closest template in `templates/`
(`docs/templates.md`), each already passing the check:

| Yen says… | Template | What it is |
|---|---|---|
| "an app for…", nothing fits | `blank` | one page of notes: add, list, remove |
| "a shopping list", "a to-do list", "the chores" | `list` | a shared list: add, edit, tick off, reorder |
| "a diary", "write about our day" | `journal` | dated entries, a photo each |
| "my recipes", "our books", "plants in the garden" | `collection` | cards with a photo, tags and search |
| "what's on this week", "the family calendar" | `planner` | a week view and a month view |
| "track my savings", "how much I walk", "the kids' reading" | `tracker` | log a number, see a chart |
| "a homework helper", "ask about…", "a chat" | `assistant` | an AI chat the app tells how to answer |
| "draw pictures", "make a picture of…" | `picture-maker` | describe it, draw it, keep it |
| "remind me…" | `reminders` | a due time per record, a notification at it |
| "a summary every morning", "tell me what's on today" | `daily-digest` | a morning workflow that gathers and notifies |
| "ask everyone…", "a poll", "RSVP" | `form` | a form that saves answers, a page summing them up |

**Starting from a template:**

1. Choose the new app's **id**: lowercase letters, digits, `-` and `_`,
   starting with a letter — `recipes`, `kids-reading`. It is the folder, the
   address (`/a/<id>/`) and where its data lives; renaming later is possible
   but is a Zen task (`docs/lifecycle.md`), so choose well.
2. Copy every file of `templates/<template>/` to `apps/<id>/`.
3. In `app.yaml`, set `id: <id>`, its `name`, `description` and `tile`, and
   `guide:` to this guide's edition. Leave `status: draft`.
4. Rename what is the template's own: the page tags (`blank-home` →
   `recipes-home`), each in its `customElements.define` and in `app.js`; the
   title in `index.html` and `app.js`.
5. Make it what Yen asked for: its collections and fields, its words, its
   pages.

A small template (`blank`, `list`, `tracker`, `assistant`) is copied with
`file_edit_and_commit`, all of its files in one commit. A larger one — or
anything touching many files — goes to a Zen task (`docs/zen-tasks.md`),
briefed with this guide's steps.

## An app's shape

```text
apps/<id>/
  app.yaml      the manifest (docs/manifest.md) — not served
  index.html    loads the framework and the app, holds <yen-app>
  app.js        defineApp: the pages and the title
  pages/        one file per page: a component per route
  components/   one file per reusable piece, named for its tag
  stores/       one file per kind of data: its collection, what is worked out from it
  style.css     page-wide styles, in tokens
  workflows/    what runs on the server — reminders, summaries (docs/workflows.md)
```

Nothing else: no notes, no images, no other folders. **Why:** every app has
the same shape however big it grows, so any session can find anything, and a
new feature adds files instead of tangling one.

The rules the gate holds every app to, each with its reason — the whole list
with what to do instead is `docs/check.md`:

- **A page reads and writes data only through a store** (`stores/`). One
  file says where each kind of data comes from; a change to it is made once.
- **A component takes its data as properties and raises events**; it never
  imports a store. It then shows whatever it is given and works anywhere.
- **One component per file, the file named for its tag**
  (`components/recipe-card.js` defines `<recipe-card>`). Finding a tag is
  never a search.
- **Styles in tokens**: `var(--yen-space-3)`, `var(--yen-accent)`, never
  `12px` or `#c2375b`. Light and dark, the app's theme and the platform's
  look then reach every app without an edit (`docs/design.md`).
- **A page or component lays itself out in its own `static styles`**: each
  renders in its own shadow root, which `style.css` does not reach.
  `style.css` is for page-wide rules only.
- **Records are listed with `<yen-list>`**, never `.value.map` in a
  template: the list has the loading, empty and error states built in.
- **Every `<img>` has an `alt`**: what it shows, or `alt=""` when it only
  decorates.
- **Nothing from outside the site**: no CDN, no font, no picture, no call to
  another address. Pictures come from the art library or the app's own
  files; another service is called by a workflow (`docs/integrations.md`).
- **No markup from strings** (`innerHTML`, `unsafeHTML`), **no inline
  `style="…"`**, no `eval`. The site's content policy refuses them anyway.

## Turning a request into pieces

| Yen asks… | The pieces | Read |
|---|---|---|
| "add a photo to each recipe" | a `file` field, `<yen-image-picker>`, `app.files.upload`; the collection's `version` raised with its migration if records change shape | `framework.md`, `ai.md`, `data.md` |
| "let only me see it" | `scope: private` on the collection; or, for the whole app, Yen makes it private on the admin page | `data.md`, `access.md` |
| "the kids can see it, only we change it" | `roles: {read: [everyone], write: [household]}` | `access.md` |
| "share this one list with Grandma" | a record share, `PUT …/_share` | `access.md` |
| "remind me the night before" | a `time` field, `schedules: - from: … at: … before: 1d run: …`, a workflow calling `ctx.notify`, `notify: true` | `workflows.md`, `notifications.md` |
| "send me a summary every Sunday" | `schedules: - every: weekly sun 18:00 run: …`, a workflow, `ctx.notify` | `workflows.md` |
| "tell me when the kids finish a chore" | `schedules: - when: {collection: chores, does: update, where: {done: true}} run: …` | `workflows.md` |
| "draw a picture for each recipe" | `ai: true`, `app.ai.image`, a `file` field; many at once in a workflow with `deadline: 10m` | `ai.md` |
| "a chat that knows our recipes" | `ai: true`, `app.ai.ask(question, {from: "recipes"})`, `search:` on the collection | `ai.md` |
| "the weather in the morning note" | `fetch: [the host]`, `ctx.fetch` in the workflow, the key on the admin page | `integrations.md` |
| "make it pink", "rounder buttons" | `theme:` in `app.yaml` | `design.md` |
| "a nicer picture on the tile" | a library picture by id; a new one is a Zen task in the house style | `library.md` |
| "put it on my phone" | nothing: every app installs from the bell | `notifications.md` |
| "I want it in a spreadsheet" | `GET /api/v1/export/<app>?format=csv&coll=…` from a task | `lifecycle.md` |

## The manifest

`app.yaml` names the app and declares what it stores. In full in
`docs/manifest.md`; the parts every app has:

```yaml
id: recipes
name: Recipes
description: What we cook, with a photo of each.
tile: library/tiles/pot-sage       # a library picture, by id (docs/library.md)
status: draft                      # draft until Yen says it is ready
guide: 1                           # this guide's edition
collections:
  recipes:
    scope: shared                  # every account that may open the app reads them; `private`: each person's own
    fields:
      title: { type: text, required: true, max: 120 }
      serves: { type: number, min: 1, default: 2 }
      photo: { type: file }
      tags: { type: list, of: text }
    search: [title, tags]          # what full-text search and app.ai.ask read
    index: [serves]                # fields filtered or ordered on often
```

**Fields** (`docs/data.md`): `text` (`long: true` for a paragraph),
`number`, `bool`, `time` (milliseconds), `date` (`YYYY-MM-DD`), `file`,
`list` (`of:` its item type), `object`, `ref` (`coll:` another collection).
`required`, `min`, `max`, `default`. The site holds every write to them: a
field not declared is refused. No field may be named `id`, `owner`,
`version`, `created_ms`, `updated_ms` or `pending`.

**Changing a collection's shape later** — a field renamed, split, or given a
new meaning — raises its `version` and ships a migration for the step
(`workflows/migrate-<coll>-<n>.js`); adding an optional field needs neither
(`docs/workflows.md`, "Changing a collection's shape").

## Pages, stores, components and forms

Every one is written as `docs/framework.md` shows, with the design system's
components (`docs/design.md`: `<yen-page>`, `<yen-list>`, `<yen-card>`,
`<yen-button>`, `<yen-fab>`, `<yen-form>`, fields, `<yen-chart>`,
`<yen-calendar>`, `<yen-chat>`, `<yen-draw>`, …). The essentials:

```js
// stores/recipes.js — the collection as a signal, and what is worked out from it
import { app, computed } from "/_/yen.js";
export const recipes = app.collection("recipes", { order: "title" });
export const quick = computed(() => recipes.value.filter((r) => r.minutes <= 30));
```

```js
// pages/home.js — a page: reads the store, lists with <yen-list>
import { YenElement, html } from "/_/yen.js";
import { recipes } from "../stores/recipes.js";
import "../components/recipe-card.js";

export class RecipesHome extends YenElement {
  render() {
    return html`<yen-page heading="Recipes">
      <yen-list .items=${recipes.value} .item=${(r) => html`<recipe-card .recipe=${r}></recipe-card>`}
                ?loading=${recipes.loading.value} empty="No recipes yet" empty-icon="utensils"></yen-list>
      ${recipes.mayAdd ? html`<yen-fab icon="plus" href="new">Add a recipe</yen-fab>` : ""}
    </yen-page>`;
  }
}
customElements.define("recipes-home", RecipesHome);
```

- **Hide what the person may not do** (`mayAdd`, `mayEdit(record)`) rather
  than letting it fail.
- **Every address inside the app is relative** (`href="new"`, `r/${id}`):
  the page carries `<base href="/a/<id>/">`.
- **Writes work offline**: a record added with no connection shows at once,
  marked `pending`, and is sent when it is back. Say so where it matters
  (`(not sent yet)`).
- **Words are Yen's family's**: plain, warm, short. "Add a recipe", not
  "Create entity". English throughout.

## Look

The design system is the platform's (`docs/design.md`; every component and
state is on `/_design`). An app chooses its accent and a few named tokens in
`theme:`; the gate refuses a theme too pale to read. Art comes from the
library by id (`docs/library.md`): `tile:`, `<yen-empty art="…">`. A picture
the library lacks is drawn in the house style and added by a Zen task — never
an image file in the app's folder.

## Doing things on its own

Reminders, summaries and reactions to a change are workflows run by jobs
(`docs/workflows.md`); a notification is `ctx.notify` from one, and the app
says `notify: true` (`docs/notifications.md`). Send inside `ctx.once`, so a
retry never sends twice.

## AI, pictures and files

`ai: true`, then `app.ai.chat`, `app.ai.image`, `app.ai.ask` from a page and
`ctx.ai` from a workflow (`docs/ai.md`). A chat's instructions — how it
answers, who it is for — are the app's, in its store. A picture takes half a
minute to a few: show its progress. Files are kept with `app.files.upload`
and named by id in a `file` field.

## Who sees what

An app is opened by every approved account unless Yen makes it private on
the admin page (`docs/access.md`). Inside it: `scope: private` collections
are each person's own; `roles:` limit a shared collection's readers and
writers to groups; one record can be shared with named people. A draft
opens for the admins alone.

## Drafts and going live

A new app is `status: draft`: Yen (and John) see it; nobody else does, and
it is not on anyone's landing page. **Ask Yen before taking it out of
draft**; going live is `status: live`, one edit, and every approved account
then sees it unless she made it private. Making it private, or sharing it
with a group, is hers to do on the admin page — or a task's, at her word
(`docs/calling-the-api.md`).

## Editing, and what the gate checks

Every change is a commit through `file_edit_and_commit`, which the gate —
yenbot itself — checks before it lands: **live** means it is on the site
within a second, open pages reloading; **refused** names the file, the line
and what to do instead, and nothing moved (`docs/publishing.md`,
`docs/check.md`). Fix what it names and call again. A change too big for a
handful of files goes to a Zen task.

## After the change

1. **Ask Yen to look**, at the address (`https://yenbot.com.au/a/<id>/`),
   and to paste a screenshot if something seems off.
2. **Look before guessing** when she says it is wrong: `log_grep` on
   `yenbot/<id>` shows every browser error and workflow run
   (`docs/logs.md`); the API shows the records (`docs/calling-the-api.md`).
3. **Note what was decided** in the journal, with why (`docs/journal.md`).

## The checklist

Confirm each to Yen, in plain words, before saying it is done:

- [ ] Built from the closest template, under its own id, `guide:` this
      edition.
- [ ] It does what she asked, in her words; nothing she did not ask for.
- [ ] Each kind of data is a collection with its fields; private where it
      should be.
- [ ] Every page reads through a store; every list is a `<yen-list>`; every
      picture has its `alt`; every style is a token.
- [ ] It is a draft until she says it is ready, and she knows how to open
      it.
- [ ] What was decided is in the journal.
