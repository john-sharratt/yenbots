// One ER matter: what it is, where it stands, its file notes, and closing it.
import { YenElement, ask, css, go, html, toast } from "/_/yen.js";
import { MATTER_KINDS, STATUSES, matters, setKind, setStatus } from "../stores/matters.js";
import { people } from "../stores/people.js";
import { KINDS, addNote, notesOfMatter, removeNote } from "../stores/notes.js";
import { removeMatter } from "../stores/journey.js";
import { showDay, today } from "../stores/days.js";
import "../components/note-adder.js";
import "../components/note-row.js";

export class MatterPage extends YenElement {
  static styles = css`
    h3 {
      font-family: var(--yen-font-heading);
      font-size: var(--yen-text-lg);
      margin-top: var(--yen-space-6);
      margin-bottom: var(--yen-space-3);
    }
    .facts {
      display: grid;
      gap: var(--yen-space-2);
      color: var(--yen-ink-2);
      margin-bottom: var(--yen-space-4);
    }
    .facts strong {
      color: var(--yen-ink);
      font-weight: var(--yen-weight-semibold);
    }
    .row {
      display: flex;
      flex-wrap: wrap;
      gap: var(--yen-space-3);
      margin-top: var(--yen-space-6);
    }
  `;

  async note(m, fields) {
    try {
      await addNote({ person: m.person, matter: m.id, ...fields });
    } catch (err) {
      toast(err.detail ?? "Couldn't save the note", { tone: "danger" });
    }
  }

  async dropNote(n) {
    if (await ask({ heading: "Delete this note?", confirm: "Delete", tone: "danger" })) await removeNote(n);
  }

  async remove(m) {
    if (await ask({ heading: "Delete this matter and its notes?", confirm: "Delete", tone: "danger" })) {
      await removeMatter(m);
      go(`p/${m.person}`, { replace: true });
    }
  }

  render() {
    const m = matters.get(this.params.id);
    if (!m) {
      return html`<yen-page heading="ER matter" back="./">
        <p>${matters.loading.value ? "Loading…" : "There's no such matter."}</p>
      </yen-page>`;
    }
    const p = people.get(m.person);
    const canEdit = matters.mayEdit(m);
    return html`<yen-page heading=${m.title} back="p/${m.person}">
      <div class="facts">
        <span><strong>About</strong> ${p?.name ?? ""}</span>
        <span><strong>Opened</strong> ${showDay(m.opened)}</span>
        ${m.closed ? html`<span><strong>Closed</strong> ${showDay(m.closed)}</span>` : ""}
      </div>
      <yen-grid size="small">
        <yen-select label="Kind" .options=${MATTER_KINDS} .value=${m.kind ?? ""} ?disabled=${!canEdit} @change=${(e) => setKind(m, e.detail)}></yen-select>
        <yen-select label="Status" .options=${STATUSES} .value=${m.status ?? "open"} ?disabled=${!canEdit} @change=${(e) => setStatus(m, e.detail)}></yen-select>
      </yen-grid>

      <h3>The matter</h3>
      ${canEdit ? html`<yen-form .collection=${matters} .record=${m} .fields=${["title", "summary", "outcome"]} submit="Save" @saved=${() => toast("Saved")}></yen-form>` : ""}

      <h3>File notes</h3>
      <note-adder .kinds=${KINDS} today=${today()} @add=${(e) => this.note(m, e.detail)}></note-adder>
      <yen-list
        divided
        .items=${notesOfMatter(m.id)}
        .item=${(n) => html`<note-row .note=${n} day=${showDay(n.day)} @remove=${(e) => this.dropNote(e.detail)}></note-row>`}
        empty="No notes on this matter yet"
        empty-icon="message"
      ></yen-list>

      ${canEdit ? html`<div class="row"><yen-button variant="danger" icon="trash" @click=${() => this.remove(m)}>Delete this matter</yen-button></div>` : ""}
    </yen-page>`;
  }
}
customElements.define("matter-page", MatterPage);
