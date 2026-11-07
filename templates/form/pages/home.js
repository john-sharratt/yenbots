// The questions; once answered, thanks and the way to the summary.
import { YenElement, css, html, toast } from "/_/yen.js";
import { DAYS, answers } from "../stores/answers.js";

export class FormHome extends YenElement {
  static properties = {
    name: { state: true },
    day: { state: true },
    rating: { state: true },
    bringing: { state: true },
    sent: { state: true },
  };

  static styles = css`
    form {
      display: grid;
      gap: var(--yen-space-4);
    }
  `;

  constructor() {
    super();
    this.name = "";
    this.day = "";
    this.rating = "";
    this.bringing = "";
    this.sent = false;
  }

  async send(e) {
    e.preventDefault();
    try {
      await answers.add({
        name: this.name.trim(),
        day: this.day,
        rating: Number(this.rating),
        ...(this.bringing.trim() ? { bringing: this.bringing.trim() } : {}),
      });
      this.sent = true;
    } catch (err) {
      toast(err.detail ?? "Couldn't send your answers", { tone: "danger" });
    }
  }

  render() {
    if (this.sent) {
      return html`<yen-page heading="Party poll">
        <yen-empty icon="circle-check" heading="Thank you!">Your answers are in.
          <yen-button slot="action" href="summary">See what everyone said</yen-button>
        </yen-empty>
      </yen-page>`;
    }
    return html`<yen-page heading="Party poll">
      <yen-button slot="actions" variant="ghost" size="small" href="summary">The answers so far</yen-button>
      <form @submit=${(e) => this.send(e)}>
        <yen-text-field label="Your name" required .value=${this.name} @input=${(e) => (this.name = e.detail)}></yen-text-field>
        <yen-select label="Which day suits you?" required placeholder="Choose a day" .options=${DAYS} .value=${this.day} @change=${(e) => (this.day = e.detail)}></yen-select>
        <yen-select
          label="How excited are you, 1 to 5?"
          required
          placeholder="Choose"
          .options=${["1", "2", "3", "4", "5"]}
          .value=${this.rating}
          @change=${(e) => (this.rating = e.detail)}
        ></yen-select>
        <yen-text-field label="What will you bring?" .value=${this.bringing} @input=${(e) => (this.bringing = e.detail)}></yen-text-field>
        <yen-button type="submit" variant="primary" icon="send">Send my answers</yen-button>
      </form>
    </yen-page>`;
  }
}
customElements.define("form-home", FormHome);
