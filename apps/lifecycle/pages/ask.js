// Talking to the app: ask about your people, or have it draft something.
import { YenElement, html } from "/_/yen.js";
import { ask, busy, clear, messages } from "../stores/assistant.js";

export class LifecycleAsk extends YenElement {
  render() {
    return html`<yen-page heading="Ask" back="./">
      <yen-button slot="actions" variant="ghost" size="small" icon="refresh" @click=${clear}>Start again</yen-button>
      <yen-chat
        heading="Ask about your people"
        placeholder="Whose probation ends this month? Draft a check-in email."
        .messages=${messages.value}
        ?busy=${busy.value}
        @send=${(e) => ask(e.detail)}
      ></yen-chat>
    </yen-page>`;
  }
}
customElements.define("lifecycle-ask", LifecycleAsk);
