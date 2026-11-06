// `YenElement` (§8.2): a Lit component that re-renders when a signal it
// read while rendering changes — a collection's records, who is signed in —
// and nothing else does. Every app's components extend it.
//
// A render that throws raises `yen-error` with what was thrown, which the
// shell shows in its "something went wrong" panel, and is thrown on for the
// error report; the component still re-renders on the next change.

import { LitElement } from "../vendor/lit-all.min.js";
import { effect } from "../vendor/signals-core.mjs";

export class YenElement extends LitElement {
  #stop = null;

  // Each render runs inside an effect, which notes every signal it reads;
  // the next change to any of them asks for the next render, and that render
  // notes them afresh.
  performUpdate() {
    if (!this.isUpdatePending) return;
    this.#stop?.();
    this.#stop = null;
    // Off the page, an update Lit still runs is followed by nothing.
    if (!this.isConnected) {
      this.#render();
      return;
    }
    let first = true;
    this.#stop = effect(() => {
      if (first) {
        first = false;
        this.#render();
      } else {
        this.requestUpdate();
      }
    });
  }

  #render() {
    try {
      super.performUpdate();
    } catch (e) {
      this.emit("yen-error", e);
      queueMicrotask(() => {
        throw e;
      });
    }
  }

  connectedCallback() {
    super.connectedCallback();
    // Signals that changed while it was off the page are read again.
    if (this.hasUpdated) this.requestUpdate();
  }

  disconnectedCallback() {
    super.disconnectedCallback();
    this.#stop?.();
    this.#stop = null;
  }

  /** Raise `name` with `detail`, through shadow roots, to whoever listens above. */
  emit(name, detail) {
    this.dispatchEvent(new CustomEvent(name, { detail, bubbles: true, composed: true }));
  }
}
