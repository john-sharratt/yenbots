// The Vietnamese test page (§8.7): every letter in both faces, at every
// size, capitals with two marks stacked at the tightest leading the tokens
// allow, typing, sorting and composing — the page to look at when a face,
// a line height or a control changes.

import { html, render } from "/_/vendor/lit-all.min.js";
import "/_/ui/components/index.js";
import { letters } from "./letters.js";

const NAMES = ["Đức", "Dung", "Ánh", "An", "Ẩn", "Bình", "Yến", "Ý", "Ơn", "Ông", "Ước", "Uyên"];

function grid(face) {
  return html`<div class="demo">
    <h3>${face === "display" ? "Fraunces, the display face" : "Be Vietnam Pro, the text face"}</h3>
    ${[false, true].map(
      (upper) => html`<div class="vi-grid ${face}">
        ${letters(upper).map((row) => html`<div class="vi-row">${row.map((l) => html`<span>${l}</span>`)}</div>`)}
      </div>`,
    )}
  </div>`;
}

function sorting() {
  const collator = new Intl.Collator("vi");
  const byCode = [...NAMES].sort();
  const byVi = [...NAMES].sort(collator.compare);
  return html`<div class="demo">
    <h3>Sorted by <code>Intl.Collator("vi")</code>, as every list is — and by code point, which is wrong</h3>
    <div class="row">
      <div class="state"><span>Vietnamese order</span><ol>${byVi.map((n) => html`<li>${n}</li>`)}</ol></div>
      <div class="state"><span>Code-point order</span><ol>${byCode.map((n) => html`<li>${n}</li>`)}</ol></div>
    </div>
  </div>`;
}

function composing() {
  const typed = "Nguyễn Thị Yến";
  const stored = typed.normalize("NFC");
  const count = (s) => [...s].length;
  return html`<div class="demo">
    <h3>Composing: what a keyboard may send, and what is stored (NFC, §7.5)</h3>
    <p>As typed: <strong>${typed}</strong> — ${count(typed)} code points.</p>
    <p>As stored: <strong>${stored}</strong> — ${count(stored)} code points.</p>
    <p>They look the same; only the second is the same bytes every time, so a search for it finds it.</p>
  </div>`;
}

render(
  html`${grid("text")}${grid("display")}${sorting()}${composing()}`,
  document.getElementById("generated"),
);
