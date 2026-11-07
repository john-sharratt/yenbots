// Every entry, the newest day first.
import { YenElement, html } from "/_/yen.js";
import { entries, photoOf, spoken } from "../stores/entries.js";
import "../components/entry-card.js";

export class JournalHome extends YenElement {
  render() {
    return html`<yen-page heading="Journal">
      <yen-list
        .items=${entries.value}
        .item=${(e) => html`<entry-card .entry=${e} day=${spoken(e.day)} thumb=${photoOf(e, { thumb: true }) ?? ""}></entry-card>`}
        ?loading=${entries.loading.value}
        empty="Nothing written yet"
        empty-icon="bookmark"
      ></yen-list>
      ${entries.mayAdd ? html`<yen-fab icon="plus" href="new">Write today</yen-fab>` : ""}
    </yen-page>`;
  }
}
customElements.define("journal-home", JournalHome);
