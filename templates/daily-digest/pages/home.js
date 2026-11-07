// What is coming up, and the morning note sent now to see it.
import { YenElement, html, toast } from "/_/yen.js";
import { coming, items, sendNow, when } from "../stores/items.js";

export class DigestHome extends YenElement {
  static properties = { sending: { state: true } };

  async send() {
    this.sending = true;
    try {
      const named = await sendNow();
      toast(named ? `Sent: ${named} things today` : "Nothing is on today, so nothing was sent");
    } catch (e) {
      toast(e.detail ?? "Couldn't send it", { tone: "danger" });
    } finally {
      this.sending = false;
    }
  }

  render() {
    return html`<yen-page heading="Today">
      <yen-button slot="actions" variant="ghost" size="small" icon="send" ?loading=${this.sending} @click=${() => this.send()}>Send this morning's note</yen-button>
      <yen-list
        divided
        .items=${coming.value}
        .item=${(i) => html`<yen-list-item headline=${i.title} supporting=${when(i.due_ms)}>
          <yen-icon-button slot="end" icon="trash" label="Delete ${i.title}" @click=${() => items.remove(i.id)}></yen-icon-button>
        </yen-list-item>`}
        ?loading=${items.loading.value}
        empty="Nothing coming up"
        empty-icon="sun"
      ></yen-list>
      ${items.mayAdd ? html`<yen-fab icon="plus" href="new">Add something</yen-fab>` : ""}
    </yen-page>`;
  }
}
customElements.define("digest-home", DigestHome);
