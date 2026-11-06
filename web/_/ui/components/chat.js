// `<yen-chat>` (§8.5): a conversation with the AI — what was asked, what it
// answered, its answer arriving as it is written — and the box to ask in.
// The page holds the messages and sends them (`app.ai`, §14); the panel
// shows them and raises `send` with the words to send.
//
//   <yen-chat .messages=${this.messages} ?busy=${this.asking} placeholder="Ask about your recipes"
//             @send=${(e) => this.ask(e.detail)}></yen-chat>
//
// A message is `{role: "user" | "assistant", text, pending}`; `pending` is
// an answer still arriving. Enter sends, Shift+Enter starts a new line, and
// a word still being composed — Vietnamese typed with Telex — is never sent
// half-made.

import { css, html, repeat } from "../../vendor/lit-all.min.js";
import { YenElement } from "../../framework/element.js";
import { base, button, control } from "../styles.js";
import "./empty.js";
import "./icon.js";

export class YenChat extends YenElement {
  static properties = {
    messages: { attribute: false },
    busy: { type: Boolean },
    placeholder: { type: String },
    heading: { type: String },
    draft: { state: true },
  };

  static styles = [
    base,
    button,
    control,
    css`
      :host {
        display: flex;
        flex-direction: column;
        height: min(36rem, 75dvh);
        background: var(--yen-surface);
        border: 1px solid var(--yen-line);
        border-radius: var(--yen-radius-3);
        overflow: hidden;
      }
      .log {
        flex: 1;
        overflow-y: auto;
        padding: var(--yen-space-4);
        display: flex;
        flex-direction: column;
        gap: var(--yen-space-3);
        overscroll-behavior: contain;
      }
      .msg {
        max-width: 85%;
        padding: var(--yen-space-2) var(--yen-space-3);
        border-radius: var(--yen-radius-3);
        white-space: pre-wrap;
        overflow-wrap: anywhere;
        line-height: var(--yen-leading);
        animation: in var(--yen-duration-2) var(--yen-ease-out);
      }
      .msg.user {
        align-self: flex-end;
        background: var(--yen-accent);
        color: var(--yen-on-accent);
        border-bottom-right-radius: var(--yen-radius-1);
      }
      .msg.assistant {
        align-self: flex-start;
        background: var(--yen-surface-2);
        color: var(--yen-ink);
        border-bottom-left-radius: var(--yen-radius-1);
      }
      .typing {
        display: inline-flex;
        gap: 0.25rem;
        padding: 0.35rem 0;
      }
      .typing span {
        width: 0.45rem;
        height: 0.45rem;
        border-radius: 50%;
        background: var(--yen-ink-2);
        animation: bob 1s var(--yen-ease-in-out) infinite;
      }
      .typing span:nth-child(2) {
        animation-delay: 0.15s;
      }
      .typing span:nth-child(3) {
        animation-delay: 0.3s;
      }
      form {
        display: flex;
        align-items: flex-end;
        gap: var(--yen-space-2);
        padding: var(--yen-space-3);
        border-top: 1px solid var(--yen-line);
        background: var(--yen-surface);
      }
      textarea.control {
        flex: 1;
        min-height: var(--yen-control-height);
        max-height: 10rem;
        field-sizing: content;
        resize: none;
      }
      .send {
        width: var(--yen-control-height);
        min-width: var(--yen-control-height);
        padding: 0;
        border-radius: var(--yen-radius-pill);
      }
      @keyframes in {
        from {
          opacity: 0;
          transform: translateY(0.4rem);
        }
      }
      @keyframes bob {
        0%,
        80%,
        100% {
          transform: translateY(0);
          opacity: 0.5;
        }
        40% {
          transform: translateY(-0.25rem);
          opacity: 1;
        }
      }
      @media (prefers-reduced-motion: reduce) {
        .typing span {
          animation: none;
        }
      }
    `,
  ];

  constructor() {
    super();
    this.messages = [];
    this.busy = false;
    this.draft = "";
    this.placeholder = "Ask anything";
  }

  updated(changed) {
    if (changed.has("messages")) {
      const log = this.renderRoot.querySelector(".log");
      if (log) log.scrollTop = log.scrollHeight;
    }
  }

  send(e) {
    e?.preventDefault();
    const text = this.draft.trim();
    if (!text || this.busy) return;
    this.draft = "";
    this.emit("send", text);
  }

  keyed(e) {
    if (e.key === "Enter" && !e.shiftKey && !e.isComposing && e.keyCode !== 229) this.send(e);
  }

  bubble(m) {
    const body =
      m.pending && !m.text
        ? html`<span class="typing" role="status" aria-label="Writing an answer"><span></span><span></span><span></span></span>`
        : m.text;
    return html`<div class="msg ${m.role === "user" ? "user" : "assistant"}" aria-busy=${m.pending ? "true" : "false"}>${body}</div>`;
  }

  render() {
    const messages = this.messages ?? [];
    return html`<div class="log" role="log" aria-live="polite" aria-label=${this.heading ?? "Conversation"}>
        ${messages.length === 0
          ? html`<yen-empty icon="sparkles" heading=${this.heading ?? "Ask me anything"}><slot></slot></yen-empty>`
          : repeat(messages, (m, i) => m.id ?? i, (m) => this.bubble(m))}
      </div>
      <form @submit=${this.send}>
        <textarea
          class="control"
          rows="1"
          aria-label="Your message"
          placeholder=${this.placeholder}
          .value=${this.draft}
          @input=${(e) => {
            e.stopPropagation();
            this.draft = e.target.value;
          }}
          @keydown=${this.keyed}
        ></textarea>
        <button class="btn primary send" type="submit" aria-label="Send" ?disabled=${this.busy || !this.draft.trim()}>
          <yen-icon name="send"></yen-icon>
        </button>
      </form>`;
  }
}

customElements.define("yen-chat", YenChat);
