// `<yen-list>` (§8.5): records, one to a row, each drawn by the page —
// skeleton rows while the first load runs, an empty state when there are
// none, and a new record easing in where it lands.
//
//   <yen-list .items=${recipes.value} .item=${(r) => html`<recipe-card .recipe=${r}></recipe-card>`}
//             ?loading=${recipes.loading.value} empty="No recipes yet" empty-icon="utensils">
//     <yen-button slot="empty-action" href="new">Add the first</yen-button>
//   </yen-list>
//
// A record is known by its `id`, so the rows a change leaves alone stay as
// they are; `.key` names another way to tell them apart.

import { css, html, repeat } from "../../vendor/lit-all.min.js";
import { YenElement } from "../../framework/element.js";
import { base } from "../styles.js";
import "./empty.js";
import "./skeleton.js";

export class YenList extends YenElement {
  static properties = {
    items: { attribute: false },
    item: { attribute: false },
    key: { attribute: false },
    loading: { type: Boolean },
    empty: { type: String },
    emptyIcon: { type: String, attribute: "empty-icon" },
    divided: { type: Boolean, reflect: true },
  };

  static styles = [
    base,
    css`
      :host {
        display: block;
      }
      ul {
        list-style: none;
        margin: 0;
        padding: 0;
        display: grid;
        gap: var(--yen-space-3);
      }
      :host([divided]) ul {
        gap: 0;
        background: var(--yen-surface);
        border: 1px solid var(--yen-line);
        border-radius: var(--yen-radius-3);
        overflow: hidden;
      }
      :host([divided]) li + li {
        border-top: 1px solid var(--yen-line);
      }
      li {
        animation: enter var(--yen-duration-3) var(--yen-ease-out) both;
      }
      .waiting {
        display: grid;
        gap: var(--yen-space-3);
      }
      @keyframes enter {
        from {
          opacity: 0;
          transform: translateY(0.5rem) scale(0.98);
        }
      }
    `,
  ];

  constructor() {
    super();
    this.items = [];
    this.item = (r) => html`${r?.title ?? r?.name ?? r?.id ?? ""}`;
    this.key = (r) => r?.id;
    this.loading = false;
    this.empty = "Nothing here yet";
    this.divided = false;
  }

  render() {
    const items = this.items ?? [];
    if (items.length === 0 && this.loading) {
      return html`<div class="waiting" role="status" aria-label="Loading">
        <yen-skeleton shape="block"></yen-skeleton>
        <yen-skeleton shape="block"></yen-skeleton>
        <yen-skeleton shape="block"></yen-skeleton>
      </div>`;
    }
    if (items.length === 0) {
      return html`<yen-empty icon=${this.emptyIcon || "list"} heading=${this.empty}>
        <slot name="empty"></slot>
        <slot name="empty-action" slot="action"></slot>
      </yen-empty>`;
    }
    return html`<ul>
      ${repeat(items, this.key, (r, i) => html`<li>${this.item(r, i)}</li>`)}
    </ul>`;
  }
}

customElements.define("yen-list", YenList);
