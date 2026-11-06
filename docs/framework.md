# The framework

How an app's pages are written: what each file is, what the framework does
for every app without being asked, and the whole of `/_/yen.js`. The design
is `candle/docs/yenbot_app_host.md` §8; the API every call here goes through
is [`api/index.md`](api/index.md).

There is no build step. An app is plain HTML, CSS and ES modules, and what
is written is what the browser runs on the next load.

## An app's files

```text
apps/recipes/
  app.yaml          the manifest: name, collections, fields (not served)
  index.html        loads the framework and the app, and holds <yen-app>
  app.js            defineApp: the pages and the title
  pages/            one file per page: a component per route
  components/       one file per reusable piece, used by pages
  stores/           one file per kind of data: its collection, its derived signals
  style.css         the app's own layout
  workflows/        the server's functions (not served)
```

Every app has this shape however large it grows, because the shape is what
keeps it simple: a new feature adds files, it never turns one file into a
tangle. The rules, each with its reason:

- **A page reads and writes data only through a store.** Then one file says
  where each kind of data comes from, and a change to it is made once.
- **A component takes its data as properties and raises events**; it never
  reaches into a store. Then it can be used anywhere, and shows whatever it
  is given.
- **One component per file, named for its tag.** Then the file a tag lives in
  is never a search.

## `index.html`

```html
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Recipes</title>
  <link rel="stylesheet" href="style.css">
  <script type="module" src="/_/yen.js"></script>
  <script type="module" src="app.js"></script>
</head>
<body>
  <yen-app></yen-app>
</body>
</html>
```

The site serves it for every page of the app — `/a/recipes/`, and a deep link
such as `/a/recipes/r/7` too — and adds `<base href="/a/recipes/">` to its
head, so `app.js`, `style.css` and a link to `r/7` mean the same from every
page. Write every address inside the app relative, with no leading slash. It
adds the platform's look (`/_/yen.css`) and the app's theme there too, ahead
of `style.css` ([`design.md`](design.md)).

## `app.js`

```js
import { defineApp } from "/_/yen.js";
import "./pages/home.js";
import "./pages/recipe.js";
import "./pages/new.js";

defineApp({
  title: "Recipes",
  routes: {
    "": "recipes-home",
    "new": "recipe-new",
    "r/:id": "recipe-page",
  },
});
```

Each route is a pattern relative to the app — `""` for its first page,
`:name` for a part of the address the page is given — and the tag of the page
shown for it. The first route that fits wins, so write a fixed route
(`r/new`) before the one with a parameter (`r/:id`). Import every page here:
a tag is a page only once its file has run. The title is the manifest's
`name` when `defineApp` gives none.

## A store

```js
// stores/recipes.js
import { app, computed } from "/_/yen.js";

export const recipes = app.collection("recipes", { order: "title" });
export const quick = computed(() => recipes.value.filter((r) => r.minutes <= 30));
```

`app.collection(name, {where, order})` is the collection as a signal: its
`.value` is the records, in order, and anything that read it while rendering
renders again when one changes — on this page, on another person's phone
through the change feed, or when a write made offline is sent. `where` is the
API's language (`minutes <= 30 and kind = soup`, [`api/data.md`](api/data.md));
`order` is a field, `-field` for descending, creation order when absent. Two
stores asking the same are one collection.

A **record** is its fields, with the record's own names beside them:

| Name | |
|---|---|
| `id` | its id; `pending-<n>` until a record made offline is sent |
| `owner` | the account that made it |
| `version` | how many writes it has had; 0 until a record made offline is sent |
| `created_ms`, `updated_ms` | when |
| `pending` | `true` while a write of it waits to be sent |

No field may take one of these names; the manifest is refused if one does.

| On a collection | |
|---|---|
| `.value` | the records |
| `.get(id)` | one record, or `undefined` |
| `.add(fields)` | add a record: answers it, or `null` when it waits for the connection; throws the site's refusal |
| `.update(id, fields)` | change those fields; a field set to `null` is removed |
| `.remove(id)` | delete it |
| `.search(words, limit)` | full-text search over the collection's `search:` fields, best first, accents ignored |
| `.sum({group, of, where})` | counts, and the sum, average, least and greatest of `of`, grouped |
| `.loading`, `.error`, `.offline`, `.truncated` | signals: still loading; what went wrong; showing what was last loaded because the site is out of reach; more than 2,000 records |
| `.mayAdd` | whether the caller may add a record |
| `.mayEdit(record)` | whether the caller may change it |
| `.reload()` | read it all again |

**Hide what the caller may not do** — `mayAdd` and `mayEdit` say — rather
than offering it and failing. The site refuses it anyway.

## A page

```js
// pages/home.js
import { YenElement, html } from "/_/yen.js";
import { recipes } from "../stores/recipes.js";
import "../components/recipe-card.js";

export class RecipesHome extends YenElement {
  render() {
    return html`<yen-page heading="Recipes">
      <yen-list
        .items=${recipes.value}
        .item=${(r) => html`<recipe-card .recipe=${r}></recipe-card>`}
        ?loading=${recipes.loading.value}
        empty="No recipes yet"
        empty-icon="utensils"
      ></yen-list>
      ${recipes.mayAdd ? html`<yen-fab icon="plus" href="new">Add a recipe</yen-fab>` : ""}
    </yen-page>`;
  }
}
customElements.define("recipes-home", RecipesHome);
```

A page is a `YenElement` — a [Lit](https://lit.dev) component that renders
again when a signal it read changes — built from the design system's
components ([`design.md`](design.md)). A render that throws is reported and
shows the shell's "Something went wrong" panel, saying what. A page with
parameters reads them from `this.params`, and the address's query string
(`?tag=soup`) from `this.query`, a `URLSearchParams`:

```js
// pages/recipe.js
import { YenElement, html } from "/_/yen.js";
import { recipes } from "../stores/recipes.js";

export class RecipePage extends YenElement {
  render() {
    const r = recipes.get(this.params.id);
    if (!r) return html`<p>${recipes.loading.value ? "Loading…" : "There's no such recipe."}</p>`;
    return html`<h2>${r.title}</h2><p>Serves ${r.serves}</p>`;
  }
}
customElements.define("recipe-page", RecipePage);
```

A link inside the app (`<a href="r/7">`) moves to its page without loading
it; back and forward follow. To move from code — after a save, say — call
`go("r/7")`, or `go("./", {replace: true})` in place of the page shown.

## A component

```js
// components/recipe-card.js
import { YenElement, html, css } from "/_/yen.js";

export class RecipeCard extends YenElement {
  static properties = { recipe: { attribute: false } };
  static styles = css`a { display: block; }`;

  render() {
    const r = this.recipe;
    return html`<a href="r/${r.id}">${r.title}${r.pending ? " (not sent yet)" : ""}</a>`;
  }
}
customElements.define("recipe-card", RecipeCard);
```

It takes its data as properties and raises events with `this.emit(name,
detail)`, which the page listens to with `@name=${…}`. Its styles are in the
design system's tokens, never literal colours or sizes. `/_/yen.js` also
gives a component Lit's `repeat`, `classMap`, `live`, `ifDefined` and
`when`; and a page `toast(words)` and `ask({heading, …})`.

## A form

`<yen-form>` builds a form from a collection's fields, adds a record or
changes one, and shows what the site refused beside the field it names:

```js
// pages/new.js
import { YenElement, html, go } from "/_/yen.js";
import { recipes } from "../stores/recipes.js";

export class RecipeNew extends YenElement {
  render() {
    return html`<yen-form
      .collection=${recipes}
      .fields=${["title", "serves", "tags"]}
      submit="Add it"
      @saved=${(e) => go(e.detail ? `r/${e.detail.id}` : "./")}
    ></yen-form>`;
  }
}
customElements.define("recipe-new", RecipeNew);
```

| Property | |
|---|---|
| `.collection` | the collection it writes |
| `.record` | the record it changes; a new one when absent |
| `.fields` | which fields, in order; every field it can build when absent |
| `submit` | the button's words; "Save" when absent |

It raises `saved` with the record — or with `null` for one kept on the
device until the connection is back, which the form says to the person.

| Field type | Control |
|---|---|
| `text` | a line; a paragraph when `long: true` |
| `number` | a number |
| `bool` | a checkbox |
| `time` | a date and time, in the device's time zone |
| `date` | a date |
| `list` of `text` | words, separated by commas |

A `file`, an `object`, a `ref` and a list of anything else are not built into
a form; the page builds those itself. A field's label is its name: `due_ms`
is "Due", `shopping-list` is "Shopping list".

## What every app gets without asking

- **The shell**, `<yen-app>`: the app's title, the way back to the landing
  page, the account's picture; then the page the address names.
- **Sign-in handled**: a visitor not signed in is offered "Sign in with
  Google"; an account not yet approved is told Yen will let them in; an app
  not shared with the caller says so. A page never sees any of them.
- **Offline writes**: every write is kept in the browser before it is sent.
  One made with no connection shows at once, marked `pending`, and is sent
  when the connection is back, in the order it was made, each with the
  version it was made against. The shell says how many wait.
- **Conflicts shown, never overwritten**: a write sent later to a record
  someone else changed meanwhile is held, and the shell offers the person
  "Keep mine" — send it again over the newer version — or "Drop it".
- **Offline reading**: with no connection a collection shows what it last
  loaded.
- **Live lists**: the change feed keeps every collection current; a list
  another person changes updates on its own.
- **Every error reported**: an uncaught error, a rejected promise and a
  failed call are posted to the site and written to the app's log,
  `yenbot/<app>` — read it with `log_grep` before changing anything. An
  error from the app's own files also shows a "Something went wrong" panel
  that says what, with "Try again".
- **Reload on publish**: an open page reloads itself when an edit goes live,
  or when it next shows if it was hidden.
- **Text in NFC**: every string written is normalised, so the same
  Vietnamese word is always the same bytes.

## Where it lives

| File | |
|---|---|
| `web/_/yen.js` | the one import: everything above |
| `web/_/framework/element.js` | `YenElement` |
| `web/_/framework/sdk.js`, `collection.js` | the app, its collections |
| `web/_/framework/records.js` | a record's shape, queued writes laid over records, ordering |
| `web/_/framework/queue.js` | offline writes, held writes |
| `web/_/framework/api.js` | every call to the site |
| `web/_/framework/router.js`, `shell.js` | pages and the shell |
| `web/_/framework/fields.js`, `form.js` | forms |
| `web/_/framework/feed.js` | the change feed and the reload signal |
| `web/_/framework/report.js`, `boot.js` | error reports; the page's app, wired to the browser |
| `web/_/framework/idb.js` | what a page keeps between visits |
| `web/_/ui/`, `web/_/yen.css` | the design system ([`design.md`](design.md)) |
| `web/_/vendor/` | Lit 3.2.1 with its directives and `@preact/signals-core` 1.8.0, pinned byte for byte by a test |

The platform's files are embedded in the binary, so a change to them is a
release. `tests/framework/*.test.js` test every module that needs no browser,
in QuickJS, under `cargo test`; what needs one is shown on `/_design`.
