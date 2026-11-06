// The art library's page (`/_library`, §15.3): every picture, filtered by
// kind, colour and tag, each with its id to copy. It reads
// `GET /api/v1/library` once and filters in the page, so a filter answers at
// once; the address keeps the filters (`?kind=tiles&colour=coral&tag=bell`),
// so a view can be linked.

import { PALETTE } from "../ui/palette.js";

const KINDS = ["tiles", "illustrations", "empty", "backgrounds", "stickers"];
const params = new URLSearchParams(location.search);
const state = {
  kind: params.get("kind") ?? "",
  colour: params.get("colour") ?? "",
  tag: params.get("tag") ?? "",
};

const $ = (id) => document.getElementById(id);

/** A row of toggle buttons, one chosen at most; "" is "any". */
function choices(row, values, key, label) {
  row.textContent = "";
  for (const v of ["", ...values]) {
    const b = document.createElement("button");
    b.type = "button";
    b.className = "choice";
    if (key === "colour" && v) {
      const dot = document.createElement("span");
      dot.className = "dot";
      dot.style.background = `var(--yen-${v})`;
      b.append(dot);
    }
    b.append(v || `Any ${label}`);
    b.setAttribute("aria-pressed", String(state[key] === v));
    b.addEventListener("click", () => {
      state[key] = v;
      draw();
    });
    row.append(b);
  }
}

/** The smallest size at least `want` wide, or the largest. */
function sizeFor(p, want) {
  const asc = [...p.sizes].sort((a, b) => a.width - b.width);
  return asc.find((s) => s.width >= want) ?? asc[asc.length - 1];
}

function card(p) {
  const li = document.createElement("li");
  li.className = `picture ${p.kind}`;
  const b = document.createElement("button");
  b.type = "button";
  b.title = `Copy ${p.id}`;
  const img = document.createElement("img");
  const s = sizeFor(p, 300);
  img.src = s.url;
  img.width = s.width;
  img.height = s.height;
  img.loading = "lazy";
  img.alt = p.tags.join(", ");
  const id = document.createElement("code");
  id.textContent = p.id;
  const tags = document.createElement("span");
  tags.className = "tags";
  tags.textContent = p.tags.join(" · ");
  const swatch = document.createElement("span");
  swatch.className = "dot";
  swatch.style.background = p.colour;
  swatch.title = `${p.colour}, nearest ${p.palette}`;
  const words = document.createElement("span");
  words.className = "words";
  words.append(id, tags);
  b.append(img, words, swatch);
  b.addEventListener("click", async () => {
    try {
      await navigator.clipboard.writeText(p.id);
      b.dataset.copied = "Copied";
      setTimeout(() => delete b.dataset.copied, 1200);
    } catch {
      // A page without clipboard access still shows the id.
    }
  });
  li.append(b);
  return li;
}

let all = [];

function draw() {
  choices($("kinds"), KINDS, "kind", "kind");
  choices($("colours"), PALETTE, "colour", "colour");
  const tag = state.tag.trim().toLowerCase();
  const shown = all.filter(
    (p) =>
      (!state.kind || p.kind === state.kind) &&
      (!state.colour || p.palette === state.colour) &&
      (!tag || p.tags.some((t) => t.startsWith(tag)) || p.id.includes(tag)),
  );
  const list = $("pictures");
  list.textContent = "";
  list.append(...shown.map(card));
  $("count").textContent =
    all.length === 0 ? "The library has no pictures yet." : `${shown.length} of ${all.length} pictures`;
  const q = new URLSearchParams();
  for (const [k, v] of Object.entries(state)) if (v.trim()) q.set(k, v.trim());
  history.replaceState(null, "", q.size ? `?${q}` : location.pathname);
}

$("tag").value = state.tag;
$("tag").addEventListener("input", (e) => {
  state.tag = e.target.value;
  draw();
});
$("filters").addEventListener("submit", (e) => e.preventDefault());

try {
  const res = await fetch("/api/v1/library");
  all = (await res.json()).pictures ?? [];
} catch {
  $("count").textContent = "The library could not be read.";
}
draw();
