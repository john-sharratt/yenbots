// What a sheet and a dialog share (§8.5): the browser's own modal
// `<dialog>`, which keeps focus inside while it is open, gives it back after,
// and shuts on Escape. `open` shows and hides it; a press on the dimmed page
// around it shuts it too; however it is shut, it raises `close`.

import { YenElement } from "../framework/element.js";

export class YenModal extends YenElement {
  static properties = {
    open: { type: Boolean, reflect: true },
    heading: { type: String },
  };

  constructor() {
    super();
    this.open = false;
  }

  get dialog() {
    return this.renderRoot?.querySelector("dialog") ?? null;
  }

  show() {
    this.open = true;
  }

  close() {
    this.open = false;
  }

  updated(changed) {
    const d = this.dialog;
    if (!d || !changed.has("open")) return;
    if (this.open && !d.open) d.showModal();
    if (!this.open && d.open) d.close();
  }

  /** The `<dialog>` shut itself — by Escape, or by `close()`. */
  shut() {
    if (this.open) this.open = false;
    this.emit("close");
  }

  /** A press on the backdrop: on the dialog element itself, outside its box. */
  pressed(e) {
    if (e.target !== this.dialog) return;
    const r = this.dialog.getBoundingClientRect();
    const inside = e.clientX >= r.left && e.clientX <= r.right && e.clientY >= r.top && e.clientY <= r.bottom;
    if (!inside) this.close();
  }
}
