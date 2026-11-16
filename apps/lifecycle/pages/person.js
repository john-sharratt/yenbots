// One person: their details, where they are, their checklist, their ER
// matters and their file notes.
import { YenElement, ask, css, go, html, repeat, toast } from "/_/yen.js";
import { EMPLOYMENT, STAGES, employmentLabel, people, probationEnd, stageLabel } from "../stores/people.js";
import { addTask, progressOf, removeTask, tasks, tasksOf, toggle } from "../stores/tasks.js";
import { KINDS, addNote, notesOf, removeNote } from "../stores/notes.js";
import { matters, mattersOf, statusLabel } from "../stores/matters.js";
import { moveTo, removePerson, savePerson } from "../stores/journey.js";
import { fromNow, showDay, today } from "../stores/days.js";
import "../components/person-editor.js";
import "../components/task-row.js";
import "../components/task-adder.js";
import "../components/matter-row.js";
import "../components/note-adder.js";
import "../components/note-row.js";

export class PersonPage extends YenElement {
  static properties = { editing: { state: true }, saving: { state: true }, moving: { state: true } };

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
      line-height: var(--yen-leading);
    }
    .facts strong {
      color: var(--yen-ink);
      font-weight: var(--yen-weight-semibold);
    }
    .about {
      white-space: pre-wrap;
      line-height: var(--yen-leading);
    }
    .row {
      display: flex;
      flex-wrap: wrap;
      gap: var(--yen-space-3);
      margin-top: var(--yen-space-4);
    }
    yen-progress {
      display: block;
      margin-bottom: var(--yen-space-3);
    }
  `;

  constructor() {
    super();
    this.editing = false;
    this.saving = false;
    this.moving = false;
  }

  async save(p, fields) {
    this.saving = true;
    try {
      await savePerson(p, fields);
      this.editing = false;
      toast("Saved");
    } catch (err) {
      toast(err.detail ?? "Couldn't save it", { tone: "danger" });
    } finally {
      this.saving = false;
    }
  }

  async move(p, stage) {
    this.moving = true;
    try {
      await moveTo(p, stage);
      toast(`Moved to ${stageLabel(stage)}`);
    } catch (err) {
      toast(err.detail ?? "Couldn't move them", { tone: "danger" });
    } finally {
      this.moving = false;
    }
  }

  async note(p, fields) {
    try {
      await addNote({ person: p.id, ...fields });
    } catch (err) {
      toast(err.detail ?? "Couldn't save the note", { tone: "danger" });
    }
  }

  async dropNote(n) {
    if (await ask({ heading: "Delete this note?", confirm: "Delete", tone: "danger" })) await removeNote(n);
  }

  async remove(p) {
    if (await ask({ heading: `Delete ${p.name} and everything about them?`, confirm: "Delete", tone: "danger" })) {
      await removePerson(p);
      go("./", { replace: true });
    }
  }

  facts(p) {
    const end = probationEnd(p);
    const probation = p.probation_weeks
      ? `${p.probation_weeks} weeks${end ? `, ends ${showDay(end)}, ${fromNow(end)}` : ""}`
      : end && `Ends ${showDay(end)}, ${fromNow(end)}`;
    const rows = [
      ["Role", [p.role, p.team].filter(Boolean).join(", ")],
      ["Manager", p.manager],
      ["Location", p.location],
      ["Employment", employmentLabel(p.employment)],
      ["Started", showDay(p.start_date)],
      ["Probation", probation],
      ["Contract ends", p.contract_end && `${showDay(p.contract_end)}, ${fromNow(p.contract_end)}`],
      ["Last day", p.last_day && `${showDay(p.last_day)}, ${fromNow(p.last_day)}`],
    ].filter(([, v]) => v);
    return html`<div class="facts">
        ${repeat(
          rows,
          ([k]) => k,
          ([k, v]) => html`<span><strong>${k}</strong> ${v}</span>`,
        )}
      </div>
      ${p.notes ? html`<p class="about">${p.notes}</p>` : ""}`;
  }

  editor(p) {
    return html`<person-editor .person=${p} .employment=${EMPLOYMENT} submit="Save changes" ?saving=${this.saving} @save=${(e) => this.save(p, e.detail)}></person-editor>
      <div class="row">
        <yen-button variant="danger" icon="trash" @click=${() => this.remove(p)}>Delete this person</yen-button>
      </div>`;
  }

  render() {
    const p = people.get(this.params.id);
    if (!p) {
      return html`<yen-page heading="Person" back="./">
        <p>${people.loading.value ? "Loading…" : "There's nobody here by that link."}</p>
      </yen-page>`;
    }
    const stage = p.stage ?? "onboarding";
    const { done, total } = progressOf(p.id);
    const now = today();
    const canEdit = people.mayEdit(p);
    return html`<yen-page heading=${p.name} back="./">
      ${canEdit
        ? html`<yen-button slot="actions" size="small" variant="ghost" icon=${this.editing ? "x" : "pencil"} @click=${() => (this.editing = !this.editing)}
            >${this.editing ? "Done" : "Edit"}</yen-button
          >`
        : ""}
      ${this.editing ? this.editor(p) : this.facts(p)}

      <h3>Where they are</h3>
      <yen-select
        label="Stage"
        hint="Onboarding and Leaving add the standard checklist"
        .options=${STAGES}
        .value=${stage}
        ?disabled=${this.moving || !canEdit}
        @change=${(e) => this.move(p, e.detail)}
      ></yen-select>

      <h3>Checklist</h3>
      ${total ? html`<yen-progress .value=${done / total} label=${`${done} of ${total} done`}></yen-progress>` : ""}
      <yen-list
        divided
        .items=${tasksOf(p.id)}
        .item=${(t) => html`<task-row
          .task=${t}
          due=${t.due ? `${showDay(t.due)}, ${fromNow(t.due)}` : ""}
          ?late=${!!t.due && t.due < now}
          @toggle=${(e) => toggle(e.detail)}
          @remove=${(e) => removeTask(e.detail)}
        ></task-row>`}
        ?loading=${tasks.loading.value}
        empty="Nothing on the checklist"
        empty-icon="list"
      ></yen-list>
      ${tasks.mayAdd ? html`<task-adder @add=${(e) => addTask(p.id, stage, e.detail.title, e.detail.due)}></task-adder>` : ""}

      <h3>ER matters</h3>
      <yen-list
        divided
        .items=${mattersOf(p.id)}
        .item=${(m) => html`<matter-row .matter=${m} opened=${showDay(m.opened)} status=${statusLabel(m.status)}></matter-row>`}
        ?loading=${matters.loading.value}
        empty="No ER matters"
        empty-icon="circle-check"
      ></yen-list>
      ${matters.mayAdd
        ? html`<div class="row"><yen-button variant="secondary" icon="plus" href="matter/${p.id}">Open an ER matter</yen-button></div>`
        : ""}

      <h3>File notes</h3>
      <note-adder .kinds=${KINDS} today=${now} @add=${(e) => this.note(p, e.detail)}></note-adder>
      <yen-list
        divided
        .items=${notesOf(p.id)}
        .item=${(n) => html`<note-row .note=${n} day=${showDay(n.day)} matter=${n.matter ? (matters.get(n.matter)?.title ?? "") : ""} @remove=${(e) => this.dropNote(e.detail)}></note-row>`}
        empty="No file notes yet"
        empty-icon="message"
      ></yen-list>
    </yen-page>`;
  }
}
customElements.define("person-page", PersonPage);
