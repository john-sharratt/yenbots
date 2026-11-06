# The check

What the platform refuses in an app, and why. The design is
`candle/docs/yenbot_app_host.md` §17.4.

**Every change is checked before it lands.** `file_edit_and_commit` answers
**live** or **refused**: refused means the gate — yenbot itself — read the
change and found something on this page, and nothing moved. The answer names
the file, the line and what to do instead:

```text
the gate yenbot refused it: apps/recipes/style.css:2 uses the colour `#c2375b`; use a token: var(--yen-accent), var(--yen-ink), …
```

Fix what it names and call again. A change pushed another way — from a
laptop — is checked when it arrives, and an app it refuses keeps being served
as it was at its last good commit; `GET /api/v1/admin/apps/{id}/check`
(`api/admin-apps.md`) says why.

**Every rule is deterministic.** The same files always get the same answer,
and no rule asks a model anything. A rule either decides exactly (a file that
is not there) or matches the source as written (a literal colour): what it
refuses is always wrong. How a page *looks* is not checked: that is Yen's to
judge from a screenshot. A field a collection does not declare is not checked
here either — the API refuses it on every write.

## The rules

| Rule | Refuses | Instead |
|---|---|---|
| `manifest` | An app folder without `app.yaml`; a manifest that does not parse, names something badly, declares a reserved field, or has a theme too pale to read | Fix what the message names (`docs/design.md` for the theme) |
| `references` | A `tile` not in the library; a schedule's `run` with no `workflows/<run>.js`; `.collection("x")` for a collection `app.yaml` does not declare; a collection's `version` lowered, or raised without `workflows/migrate-<coll>-<n>.js` for each step | Name what is there, or add it |
| `guide` | A new app with no `guide:`; a changed app built to an older edition of `docs/building-an-app.md` | Read the guide, bring the app up to it, set `guide:` to its edition |
| `layout` | A file outside the shape (below); a component defined outside `pages/` or `components/`, two in one file, or one whose file is not named for its tag; a component importing a store | Keep to the shape; pass data to a component as properties |
| `tokens` | A literal colour (`#…`, `rgb(…)`, a named colour) anywhere in styles; a literal size for spacing, corners, type or line height; a font named rather than `var(--yen-font-…)` | `var(--yen-…)` (`docs/design.md`) |
| `markup` | `unsafeHTML` and its kin; `innerHTML =`, `insertAdjacentHTML`, `document.write`; `eval`, `new Function`; an `<img>` with no `alt`; an inline `style="…"`; a store's records drawn with `.value.map` | `${…}` in `html`; `alt=""` for decoration; a class or `style=${styleMap(…)}`; `<yen-list>` |
| `outside` | A script, style, picture, font or call to an address outside the site | The site's own: the framework, the library, the app's files, the API |
| `workflows` | A workflow that does not compile, imports anything but the app's other workflow files, does work as it loads, or has no default-exported function | `export default async function name(ctx, args) { … }` |

## An app's shape

```text
app.yaml  index.html  app.js  style.css
pages/        one file per page            JavaScript only
components/   one file per component, named for its tag
stores/       one file per kind of data
workflows/    the server's functions
```

Nothing else: no notes, no images — pictures come from the library.

## What passes, in one place

```js
// components/recipe-card.js — a component: its data as properties, its styles in tokens
import { YenElement, html, css } from "/_/yen.js";

export class RecipeCard extends YenElement {
  static properties = { recipe: { attribute: false } };
  static styles = css`a { display: block; padding: var(--yen-space-3); color: var(--yen-ink); }`;
  render() {
    return html`<a href="r/${this.recipe.id}"><img src=${this.recipe.photo} alt="">${this.recipe.title}</a>`;
  }
}
customElements.define("recipe-card", RecipeCard);
```

```js
// pages/home.js — a page: reads the store, lists with <yen-list>
render() {
  return html`<yen-page heading="Recipes">
    <yen-list .items=${recipes.value} .item=${(r) => html`<recipe-card .recipe=${r}></recipe-card>`}
              ?loading=${recipes.loading.value} empty="No recipes yet"></yen-list>
  </yen-page>`;
}
```

## Where it lives

| File | |
|---|---|
| `src/check/` | the check: `mod.rs` runs the rules, `rules/` has one file each |
| `src/gate/` | the gate: answers `gate/yenbot` on the gateway's bus |
| `src/apps/mod.rs` | the check on load, and the last good version kept |
