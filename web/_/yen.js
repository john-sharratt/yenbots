// The framework every app is written against (§8.2), in one import:
//
//   import { app, defineApp, YenElement, html, css, computed } from "/_/yen.js";
//
// - `YenElement`, `html`, `css`, `nothing`: components (Lit), re-rendering
//   when a signal they read changes; `repeat`, `classMap`, `live`,
//   `ifDefined` and `when`, Lit's helpers for writing them.
// - `signal`, `computed`, `effect`, `batch`, `untracked`: state.
// - `app`: the app this page is — `app.collection(name, {where, order})`,
//   `app.me`, `app.queue` — and `defineApp({routes, title})`, its pages.
// - The design system's components (`web/_/ui/components/`), every one
//   defined: `<yen-page>`, `<yen-button>`, `<yen-list>`, `<yen-sheet>`, …;
//   `<yen-app>`, the shell every app is shown in; `<yen-form>`, a form from a
//   collection's fields.
// - `toast(words)` shows a toast; `ask({heading, …})` asks in a dialog.
// - `go(address)`: show another of the app's pages.
//
// Its look is `/_/yen.css`, which the site puts on every app's pages.

export { classMap, css, html, ifDefined, live, nothing, repeat, svg, when } from "./vendor/lit-all.min.js";
export { batch, computed, effect, signal, untracked } from "./vendor/signals-core.mjs";
export { ApiError } from "./framework/api.js";
export { YenElement } from "./framework/element.js";
export { ask } from "./ui/components/dialog.js";
export { toast } from "./ui/toasts.js";
export { ICONS } from "./ui/icons.js";
import "./ui/components/index.js";
import "./framework/form.js";
import { boot } from "./framework/boot.js";

const booted = boot();

export const app = booted.sdk;

/** Show the app's page at `address`, relative to the app: `go("r/7")`; `{replace: true}` in place of this one. */
export const go = booted.go;

/** The app's pages and title: `defineApp({routes: {"": "recipes-home", "r/:id": "recipe-page"}, title: "Recipes"})`. */
export function defineApp(spec) {
  app.define(spec);
}
