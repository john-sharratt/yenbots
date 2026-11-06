// The framework every app is written against (§8.2), in one import:
//
//   import { app, defineApp, YenElement, html, css, computed } from "/_/yen.js";
//
// - `YenElement`, `html`, `css`, `nothing`: components (Lit), re-rendering
//   when a signal they read changes.
// - `signal`, `computed`, `effect`, `batch`, `untracked`: state.
// - `app`: the app this page is — `app.collection(name, {where, order})`,
//   `app.me`, `app.queue` — and `defineApp({routes, title})`, its pages.
// - `<yen-app>`: the shell every app is shown in; `<yen-form>`: a form from
//   a collection's fields.
// - `go(address)`: show another of the app's pages.

export { css, html, nothing, svg } from "./vendor/lit-core.min.js";
export { batch, computed, effect, signal, untracked } from "./vendor/signals-core.mjs";
export { ApiError } from "./framework/api.js";
export { YenElement } from "./framework/element.js";
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
