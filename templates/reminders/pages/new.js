// A new reminder: what, and when to ping.
import { YenElement, go, html } from "/_/yen.js";
import { reminders } from "../stores/reminders.js";

export class RemindersNew extends YenElement {
  render() {
    return html`<yen-page heading="A new reminder" back="./">
      <yen-form .collection=${reminders} .fields=${["title", "due_ms"]} submit="Remind me" @saved=${() => go("./", { replace: true })}></yen-form>
    </yen-page>`;
  }
}
customElements.define("reminders-new", RemindersNew);
