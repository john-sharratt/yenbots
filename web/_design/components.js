// The catalogue's components (§8.5): each one in every state it has —
// rest, hover, focus, pressed, disabled, loading, error, empty — with the
// states a pointer makes shown by `demo="…"`, and the rest by their own
// properties. A section is a function returning what its frame shows.

import { html } from "/_/vendor/lit-all.min.js";
import { ask } from "/_/ui/components/dialog.js";
import { toast } from "/_/ui/toasts.js";

const SAMPLES = [
  { src: "/_design/samples/sunrise.svg", alt: "A sun rising over hills" },
  { src: "/_design/samples/garden.svg", alt: "Leaves in a garden" },
  { src: "/_design/samples/sea.svg", alt: "Waves on the sea" },
  { src: "/_design/samples/night.svg", alt: "A moon over a quiet town" },
];

const POINTER = ["rest", "hover", "focus", "pressed"];

/** One labelled state of something. */
const state = (label, body) => html`<div class="state"><span>${label}</span>${body}</div>`;

const demo = (heading, body) => html`<div class="demo"><h3>${heading}</h3>${body}</div>`;

/** The demo a button inside it opens. */
const opener = (selector) => (e) => e.currentTarget.closest(".demo").querySelector(selector).show();

export function buttons() {
  return html`${["primary", "secondary", "ghost", "danger"].map((v) =>
      demo(
        `variant="${v}"`,
        html`<div class="row">
          ${POINTER.map((s) => state(s, html`<yen-button variant=${v} demo=${s === "rest" ? "" : s}>Save</yen-button>`))}
          ${state("disabled", html`<yen-button variant=${v} disabled>Save</yen-button>`)}
          ${state("loading", html`<yen-button variant=${v} loading>Saving</yen-button>`)}
        </div>`,
      ),
    )}
    ${demo(
      "With an icon, small, a link, and wide",
      html`<div class="row">
          <yen-button icon="plus">Add a recipe</yen-button>
          <yen-button size="small" variant="secondary" icon="share">Share</yen-button>
          <yen-button href="#buttons" variant="ghost" icon="arrow-right">A link</yen-button>
        </div>
        <yen-button wide icon="check">Done</yen-button>`,
    )}
    ${demo(
      "Icon buttons: a label is required",
      html`<div class="row">
        ${["ghost", "secondary", "primary", "danger"].map((v) =>
          state(v, html`<yen-icon-button variant=${v} icon=${v === "danger" ? "trash" : "pencil"} label="Edit"></yen-icon-button>`),
        )}
        ${state("hover", html`<yen-icon-button icon="heart" label="Love it" demo="hover"></yen-icon-button>`)}
        ${state("focus", html`<yen-icon-button icon="heart" label="Love it" demo="focus"></yen-icon-button>`)}
        ${state("pressed on", html`<yen-icon-button icon="heart" label="Love it" pressed></yen-icon-button>`)}
        ${state("disabled", html`<yen-icon-button icon="trash" label="Delete" disabled></yen-icon-button>`)}
      </div>`,
    )}
    ${demo(
      "The floating action button: shown here in the flow, it floats at the thumb's corner",
      html`<div class="row">
        ${state("rest", html`<yen-fab inline icon="plus">Add a recipe</yen-fab>`)}
        ${state("hover", html`<yen-fab inline icon="plus" demo="hover">Add a recipe</yen-fab>`)}
        ${state("compact", html`<yen-fab inline compact icon="plus">Add a recipe</yen-fab>`)}
      </div>`,
    )}`;
}

export function fields() {
  return html`${demo(
      "Text field",
      html`${state("empty", html`<yen-text-field label="Title" placeholder="Roast chicken"></yen-text-field>`)}
      ${state("filled, with a hint", html`<yen-text-field label="Title" .value=${"Apple crumble"} hint="What the family calls it"></yen-text-field>`)}
      ${state("hover", html`<yen-text-field label="Title" demo="hover" .value=${"Apple crumble"}></yen-text-field>`)}
      ${state("focus", html`<yen-text-field label="Title" demo="focus" .value=${"Apple crumble"}></yen-text-field>`)}
      ${state("error", html`<yen-text-field label="Serves" type="number" required .value=${"0"} error="Serves must be at least 1"></yen-text-field>`)}
      ${state("disabled", html`<yen-text-field label="Title" disabled .value=${"Shepherd's pie"}></yen-text-field>`)}
      ${state("search", html`<yen-text-field type="search" label="Search" placeholder="Search recipes"></yen-text-field>`)}`,
    )}
    ${demo(
      "Text area",
      html`${state("rest", html`<yen-text-area label="Method" placeholder="Soak the rice noodles…"></yen-text-area>`)}
      ${state("error", html`<yen-text-area label="Method" required error="Method is required"></yen-text-area>`)}
      ${state("disabled", html`<yen-text-area label="Method" disabled .value=${"Simmer the stock for six hours."}></yen-text-area>`)}`,
    )}
    ${demo(
      "Select",
      html`${state("a placeholder", html`<yen-select label="Kind" placeholder="Choose a kind" .options=${["Soup", "Salad", "Dessert"]}></yen-select>`)}
      ${state(
        "chosen",
        html`<yen-select
          label="Kind"
          .value=${"salad"}
          .options=${[
            { value: "soup", label: "Soup" },
            { value: "salad", label: "Salad" },
          ]}
        ></yen-select>`,
      )}
      ${state("error", html`<yen-select label="Kind" required .options=${["Soup"]} error="Kind is required"></yen-select>`)}
      ${state("disabled", html`<yen-select label="Kind" disabled .value=${"Soup"} .options=${["Soup"]}></yen-select>`)}`,
    )}
    ${demo(
      "Date and time",
      html`${state("a day, with Today", html`<yen-date-picker label="Due" .value=${"2026-11-12"}></yen-date-picker>`)}
      ${state("a time", html`<yen-date-picker label="At" type="time" .value=${"07:30"}></yen-date-picker>`)}
      ${state("both", html`<yen-date-picker label="Reminder" type="datetime" .value=${Date.UTC(2026, 10, 12, 8, 30)}></yen-date-picker>`)}
      ${state("error", html`<yen-date-picker label="Due" required error="Due is required"></yen-date-picker>`)}
      ${state("disabled", html`<yen-date-picker label="Due" disabled .value=${"2026-11-12"}></yen-date-picker>`)}`,
    )}
    ${demo(
      "Toggle and checkbox",
      html`<div class="row">
          ${state("off", html`<yen-toggle label="Remind me"></yen-toggle>`)}
          ${state("on", html`<yen-toggle label="Remind me" checked></yen-toggle>`)}
          ${state("hover", html`<yen-toggle label="Remind me" demo="hover"></yen-toggle>`)}
          ${state("focus", html`<yen-toggle label="Remind me" demo="focus" checked></yen-toggle>`)}
          ${state("disabled", html`<yen-toggle label="Remind me" disabled></yen-toggle>`)}
        </div>
        <div class="row">
          ${state("unticked", html`<yen-checkbox label="Vegetarian"></yen-checkbox>`)}
          ${state("ticked", html`<yen-checkbox label="Vegetarian" checked></yen-checkbox>`)}
          ${state("some", html`<yen-checkbox label="All the shopping" indeterminate></yen-checkbox>`)}
          ${state("hover", html`<yen-checkbox label="Vegetarian" demo="hover"></yen-checkbox>`)}
          ${state("focus", html`<yen-checkbox label="Vegetarian" demo="focus" checked></yen-checkbox>`)}
          ${state("error", html`<yen-checkbox label="I agree" error="Tick this to go on"></yen-checkbox>`)}
          ${state("disabled", html`<yen-checkbox label="Vegetarian" disabled checked></yen-checkbox>`)}
        </div>`,
    )}`;
}

export function forms(collection) {
  return demo(
    "<yen-form>: built from a collection's fields, its errors beside each",
    html`<yen-form .collection=${collection} submit="Add it"></yen-form>`,
  );
}

const RECIPES = [
  { id: "1", title: "Beef stew", serves: 4, minutes: 180 },
  { id: "2", title: "Apple crumble", serves: 6, minutes: 45 },
  { id: "3", title: "Pumpkin soup", serves: 4, minutes: 30 },
];

export function layout() {
  return html`${demo(
      "Cards",
      html`<yen-grid size="medium">
        ${state("rest", html`<yen-card><h3>Beef stew</h3><p>Serves 4 · 3 hours</p></yen-card>`)}
        ${state("a link, hover", html`<yen-card href="#layout" demo="hover"><h3>Apple crumble</h3><p>Serves 6 · 45 min</p></yen-card>`)}
        ${state("focus", html`<yen-card href="#layout" demo="focus"><h3>Pumpkin soup</h3><p>Serves 4</p></yen-card>`)}
        ${state(
          "with a picture",
          html`<yen-card href="#layout"><img slot="media" src=${SAMPLES[0].src} alt="" /><h3>Pavlova</h3><p>Dessert</p></yen-card>`,
        )}
      </yen-grid>`,
    )}
    ${demo(
      "A list of records, its loading and its empty",
      html`<yen-list
          divided
          .items=${RECIPES}
          .item=${(r) => html`<yen-list-item href="#layout" headline=${r.title} supporting=${`Serves ${r.serves}`}>
            <yen-avatar slot="start" name=${r.title}></yen-avatar>
            <span slot="end">${r.minutes} min</span>
          </yen-list-item>`}
        ></yen-list>
        ${state("loading", html`<yen-list loading></yen-list>`)}
        ${state(
          "empty",
          html`<yen-list empty="No recipes yet" empty-icon="utensils">
            <span slot="empty">Add the first, and the whole family will see it.</span>
            <yen-button slot="empty-action" icon="plus">Add a recipe</yen-button>
          </yen-list>`,
        )}`,
    )}
    ${demo(
      "List items: hover, pressed, focus",
      html`${["hover", "pressed", "focus"].map((s) =>
        state(s, html`<yen-list-item href="#layout" demo=${s} headline="Pumpkin soup" supporting="With crusty bread"></yen-list-item>`),
      )}`,
    )}
    ${demo(
      "Tabs",
      html`<yen-tabs
        label="Recipes"
        .tabs=${[
          { id: "all", label: "All", icon: "list" },
          { id: "mine", label: "Mine", icon: "user" },
          { id: "loved", label: "Loved", icon: "heart" },
        ]}
        selected="all"
      >
        <p slot="all">Every recipe in the family.</p>
        <p slot="mine">The ones you added.</p>
        <p slot="loved">The ones you love.</p>
      </yen-tabs>`,
    )}
    ${demo(
      "A page",
      html`<yen-page heading="Recipes" back="#layout">
        <yen-icon-button slot="actions" icon="search" label="Search"></yen-icon-button>
        <p>A page's heading, its way back, its actions, and its content at a readable width.</p>
      </yen-page>`,
    )}`;
}

export function overlays() {
  return html`${demo(
      "Sheet",
      html`<yen-button variant="secondary" icon="filter" @click=${opener("yen-sheet")}>Open a sheet</yen-button>
        <yen-sheet heading="Filter recipes">
          <yen-checkbox label="Vegetarian"></yen-checkbox>
          <yen-checkbox label="Under 30 minutes" checked></yen-checkbox>
          <yen-button slot="actions" variant="ghost">Clear</yen-button>
          <yen-button slot="actions">Show recipes</yen-button>
        </yen-sheet>`,
    )}
    ${demo(
      "Dialog",
      html`<div class="row">
        <yen-button variant="danger" icon="trash" @click=${opener("yen-dialog")}>Delete a recipe</yen-button>
        <yen-button
          variant="secondary"
          @click=${async () => toast((await ask({ heading: "Send it to Yen?", confirm: "Send" })) ? "Sent" : "Not sent")}
          >ask() from code</yen-button
        >
        <yen-dialog heading="Delete Beef stew?" confirm="Delete" cancel="Keep it" tone="danger">
          It goes for everyone in the family. This can't be undone.
        </yen-dialog>
      </div>`,
    )}
    ${demo(
      "Toasts",
      html`<div class="row">
        <yen-button variant="secondary" @click=${() => toast("Saved")}>Info</yen-button>
        <yen-button variant="secondary" @click=${() => toast("Recipe added", { tone: "success" })}>Success</yen-button>
        <yen-button variant="secondary" @click=${() => toast("Couldn't save — you're offline", { tone: "danger" })}>Danger</yen-button>
        <yen-button
          variant="secondary"
          @click=${() => toast("Recipe deleted", { action: { label: "Undo", run: () => toast("Put back") } })}
          >With an action</yen-button
        >
      </div>`,
    )}`;
}

export function status() {
  return html`${demo(
      "Empty state",
      html`<yen-empty icon="bell" heading="Nothing to remember">
        Add a reminder and it will ping you when it's due.
        <yen-button slot="action" icon="plus">Add a reminder</yen-button>
      </yen-empty>`,
    )}
    ${demo(
      "Skeletons",
      html`<div class="row">
          ${state("circle", html`<yen-skeleton shape="circle"></yen-skeleton>`)}
        </div>
        ${state("text", html`<yen-skeleton lines="3"></yen-skeleton>`)} ${state("block", html`<yen-skeleton shape="block"></yen-skeleton>`)}`,
    )}
    ${demo(
      "Avatars: a picture, or initials on the name's own colour",
      html`<div class="row">
        ${["Yen Sharratt", "John Sharratt", "Mary Jane Watson", "Grandma", "Zoë Brontë"].map(
          (n) => html`<yen-avatar name=${n}></yen-avatar>`,
        )}
        <yen-avatar name="Yen" size="small"></yen-avatar>
        <yen-avatar name="Yen" size="large"></yen-avatar>
        ${state("a picture that won't load", html`<yen-avatar name="Pat Morgan" src="/_design/samples/missing.png"></yen-avatar>`)}
      </div>`,
    )}
    ${demo(
      "Tags",
      html`<div class="row">
        <yen-tag>Soup</yen-tag>
        <yen-tag tone="accent" icon="heart">Loved</yen-tag>
        <yen-tag tone="success" icon="check">Done</yen-tag>
        <yen-tag tone="warning" icon="clock">Due soon</yen-tag>
        <yen-tag tone="danger" icon="alert">Overdue</yen-tag>
        <yen-tag removable>Vegetarian</yen-tag>
      </div>`,
    )}
    ${demo(
      "Progress",
      html`${state("none yet", html`<yen-progress .value=${0} label="Uploading"></yen-progress>`)}
      ${state("40%", html`<yen-progress .value=${0.4} label="Uploading"></yen-progress>`)}
      ${state("done", html`<yen-progress .value=${1} label="Uploading"></yen-progress>`)}
      ${state("how far is not known", html`<yen-progress label="Working"></yen-progress>`)}
      ${state("spinner", html`<yen-progress kind="spinner" label="Loading"></yen-progress>`)}`,
    )}`;
}

export function pictures() {
  return html`${demo(
      "Image picker",
      html`${state("rest", html`<yen-image-picker label="Photos" multiple hint="Up to 10 MB each"></yen-image-picker>`)}
      ${state("a picture dragged over it", html`<yen-image-picker label="Photo" demo="hover"></yen-image-picker>`)}
      ${state("error", html`<yen-image-picker label="Photo" error="A photo is required"></yen-image-picker>`)}
      ${state("disabled", html`<yen-image-picker label="Photo" disabled></yen-image-picker>`)}`,
    )}
    ${demo("Photo grid: tap one to see it large", html`<yen-photo-grid .photos=${SAMPLES}></yen-photo-grid>`)}`;
}

const MONTHS = [
  { label: "Jun", value: 180 },
  { label: "Jul", value: 240 },
  { label: "Aug", value: 205 },
  { label: "Sep", value: 310 },
  { label: "Oct", value: 265 },
  { label: "Nov", value: 120 },
];

export function data() {
  const money = (v) => `$${v}`;
  return html`${demo("A bar chart", html`<yen-chart label="Spent on groceries each month" .rows=${MONTHS} .format=${money}></yen-chart>`)}
    ${demo("A line chart", html`<yen-chart kind="line" label="Spent on groceries each month" .rows=${MONTHS} .format=${money}></yen-chart>`)}
    ${demo(
      "Below zero",
      html`<yen-chart
        label="Change from last week"
        .rows=${[
          { label: "Mon", value: 3 },
          { label: "Tue", value: -2 },
          { label: "Wed", value: 5 },
          { label: "Thu", value: -4 },
        ]}
      ></yen-chart>`,
    )}
    ${demo("With nothing to show", html`<yen-chart label="Spent" .rows=${[]}></yen-chart>`)}
    ${demo(
      "Calendar: today ringed, a dot where something falls, the chosen day filled",
      html`<yen-calendar .value=${"2026-11-12"} .marks=${["2026-11-03", "2026-11-12", "2026-11-20", "2026-11-28"]}></yen-calendar>`,
    )}`;
}

const CHAT = [
  { role: "user", text: "What can I cook tonight with eggs and rice?" },
  {
    role: "assistant",
    text: "Egg fried rice. Fry garlic, add yesterday's rice, push it aside and scramble two eggs, then toss with soy sauce and spring onion.",
  },
  { role: "user", text: "And something sweet after?" },
  { role: "assistant", text: "", pending: true },
];

export function ai() {
  return html`${demo(
      "AI chat: an answer still arriving",
      html`<yen-chat heading="Ask about your recipes" placeholder="Ask about your recipes" busy .messages=${CHAT}></yen-chat>`,
    )}
    ${demo(
      "AI chat: before anything is asked",
      html`<yen-chat heading="Ask about your recipes" placeholder="Ask about your recipes">
        It knows every recipe in this app.
      </yen-chat>`,
    )}
    ${demo("Drawing a picture: while it draws", html`<yen-draw busy .progress=${0.6} placeholder="A bowl of soup, watercolour"></yen-draw>`)}
    ${demo("Drawing a picture: drawn", html`<yen-draw .pictures=${SAMPLES.slice(0, 2)} placeholder="A bowl of soup, watercolour"></yen-draw>`)}
    ${demo("Drawing a picture: refused", html`<yen-draw error="The AI can't draw that. Try other words."></yen-draw>`)}`;
}
