// The chat.
import { YenElement, html } from "/_/yen.js";
import { ask, busy, clear, messages } from "../stores/chat.js";

export class AssistantHome extends YenElement {
  render() {
    return html`<yen-page heading="Homework helper">
      <yen-button slot="actions" variant="ghost" size="small" icon="refresh" @click=${clear}>Start again</yen-button>
      <yen-chat
        heading="Ask about your homework"
        placeholder="What are you working on?"
        .messages=${messages.value}
        ?busy=${busy.value}
        @send=${(e) => ask(e.detail)}
      ></yen-chat>
    </yen-page>`;
  }
}
customElements.define("assistant-home", AssistantHome);
