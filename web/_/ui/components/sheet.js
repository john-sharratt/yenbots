// `<yen-sheet heading="Filter" ?open=${…} @close=${…}>` (§8.5): a panel that
// slides up from the bottom of a phone — or stands in the middle of a wider
// screen — for a short task that does not need a page of its own. Its
// `actions` slot is its foot, for the buttons that finish it.
//
//   <yen-sheet heading="Filter" ?open=${this.filtering} @close=${() => (this.filtering = false)}>
//     …
//     <yen-button slot="actions" @click=${…}>Show recipes</yen-button>
//   </yen-sheet>

import { css, html, nothing } from "../../vendor/lit-all.min.js";
import { YenModal } from "../modal-element.js";
import { base } from "../styles.js";
import "./icon-button.js";

export class YenSheet extends YenModal {
  static styles = [
    base,
    css`
      dialog {
        position: fixed;
        inset: auto 0 0 0;
        width: 100%;
        max-width: 36rem;
        max-height: 88dvh;
        margin: 0 auto;
        padding: 0;
        border: none;
        border-radius: var(--yen-radius-3) var(--yen-radius-3) 0 0;
        background: var(--yen-surface);
        color: var(--yen-ink);
        box-shadow: var(--yen-shadow-3);
        overflow: hidden;
      }
      dialog[open] {
        display: flex;
        flex-direction: column;
        animation: up var(--yen-duration-3) var(--yen-ease-out);
      }
      dialog::backdrop {
        background: var(--yen-scrim);
        animation: fade var(--yen-duration-3) var(--yen-ease-out);
      }
      .grip {
        width: 2.5rem;
        height: 0.3rem;
        margin: var(--yen-space-2) auto 0;
        border-radius: var(--yen-radius-pill);
        background: var(--yen-line-strong);
      }
      header {
        display: flex;
        align-items: center;
        gap: var(--yen-space-2);
        padding: var(--yen-space-2) var(--yen-space-2) 0 var(--yen-space-5);
      }
      h2 {
        flex: 1;
        margin: 0;
        font-family: var(--yen-font-heading);
        font-size: var(--yen-text-xl);
        font-weight: var(--yen-weight-semibold);
        line-height: var(--yen-leading-tight);
      }
      .body {
        flex: 1;
        overflow-y: auto;
        padding: var(--yen-space-3) var(--yen-space-5) var(--yen-space-5);
        overscroll-behavior: contain;
      }
      footer {
        display: flex;
        flex-wrap: wrap;
        justify-content: flex-end;
        gap: var(--yen-space-2);
        padding: var(--yen-space-3) var(--yen-space-5) max(var(--yen-space-4), env(safe-area-inset-bottom));
        border-top: 1px solid var(--yen-line);
      }
      footer:not(:has(::slotted(*))) {
        display: none;
      }
      @media (min-width: 48rem) {
        dialog {
          inset: 0;
          margin: auto;
          height: fit-content;
          border-radius: var(--yen-radius-3);
        }
        dialog[open] {
          animation-name: pop;
        }
        .grip {
          display: none;
        }
        header {
          padding-top: var(--yen-space-3);
        }
      }
      @keyframes up {
        from {
          transform: translateY(100%);
        }
      }
      @keyframes pop {
        from {
          opacity: 0;
          transform: translateY(0.75rem) scale(0.98);
        }
      }
      @keyframes fade {
        from {
          opacity: 0;
        }
      }
    `,
  ];

  render() {
    return html`<dialog aria-label=${this.heading ?? nothing} @close=${this.shut} @click=${this.pressed}>
      <div class="grip" aria-hidden="true"></div>
      <header>
        <h2>${this.heading}</h2>
        <yen-icon-button icon="x" label="Close" @click=${() => this.close()}></yen-icon-button>
      </header>
      <div class="body"><slot></slot></div>
      <footer><slot name="actions"></slot></footer>
    </dialog>`;
  }
}

customElements.define("yen-sheet", YenSheet);
