// `<yen-dialog>` (§8.5): a question that must be answered before going on —
// "Delete this recipe?" — with its two answers. `confirm` and `cancel` are
// the buttons' words; `tone="danger"` colours the one that cannot be undone.
// It raises `confirm` or `cancel`, then `close`.
//
// `ask({heading, body, confirm, cancel, tone})` asks from code and answers
// whether the person confirmed:
//
//   if (await ask({heading: "Delete this recipe?", confirm: "Delete", tone: "danger"})) …

import { css, html, nothing } from "../../vendor/lit-all.min.js";
import { YenModal } from "../modal-element.js";
import { base } from "../styles.js";
import "./button.js";

export class YenDialog extends YenModal {
  static properties = {
    ...YenModal.properties,
    confirm: { type: String },
    cancel: { type: String },
    tone: { type: String },
  };

  static styles = [
    base,
    css`
      dialog {
        width: min(26rem, calc(100% - 2 * var(--yen-space-4)));
        padding: var(--yen-space-5);
        border: none;
        border-radius: var(--yen-radius-3);
        background: var(--yen-surface);
        color: var(--yen-ink);
        box-shadow: var(--yen-shadow-3);
      }
      dialog[open] {
        animation: pop var(--yen-duration-3) var(--yen-ease-spring);
      }
      dialog::backdrop {
        background: var(--yen-scrim);
        animation: fade var(--yen-duration-3) var(--yen-ease-out);
      }
      h2 {
        margin: 0 0 var(--yen-space-2);
        font-family: var(--yen-font-heading);
        font-size: var(--yen-text-xl);
        font-weight: var(--yen-weight-semibold);
        line-height: var(--yen-leading-tight);
      }
      .body {
        color: var(--yen-ink-2);
      }
      footer {
        display: flex;
        flex-wrap: wrap-reverse;
        justify-content: flex-end;
        gap: var(--yen-space-2);
        margin-top: var(--yen-space-5);
      }
      @keyframes pop {
        from {
          opacity: 0;
          transform: scale(0.94);
        }
      }
      @keyframes fade {
        from {
          opacity: 0;
        }
      }
    `,
  ];

  constructor() {
    super();
    this.confirm = "OK";
    this.cancel = "Cancel";
    this.tone = "primary";
  }

  answer(yes) {
    this.emit(yes ? "confirm" : "cancel");
    this.answered = true;
    this.close();
  }

  shut() {
    // Escape, or a press outside, is the cancelling answer.
    if (!this.answered) this.emit("cancel");
    this.answered = false;
    super.shut();
  }

  render() {
    return html`<dialog
      role="alertdialog"
      aria-labelledby="heading"
      aria-describedby="body"
      @close=${this.shut}
      @click=${this.pressed}
    >
      <h2 id="heading">${this.heading ?? nothing}</h2>
      <div class="body" id="body"><slot></slot></div>
      <footer>
        <yen-button variant="secondary" @click=${() => this.answer(false)}>${this.cancel}</yen-button>
        <yen-button variant=${this.tone === "danger" ? "danger" : "primary"} @click=${() => this.answer(true)}
          >${this.confirm}</yen-button
        >
      </footer>
    </dialog>`;
  }
}

customElements.define("yen-dialog", YenDialog);

/** Ask a question in a dialog; answers whether the person confirmed. */
export function ask({ heading, body = "", confirm = "OK", cancel = "Cancel", tone = "primary" }) {
  return new Promise((resolve) => {
    const d = document.createElement("yen-dialog");
    Object.assign(d, { heading, confirm, cancel, tone });
    d.textContent = body;
    let yes = false;
    d.addEventListener("confirm", () => (yes = true));
    d.addEventListener("close", () => {
      d.remove();
      resolve(yes);
    });
    document.body.append(d);
    d.show();
  });
}
