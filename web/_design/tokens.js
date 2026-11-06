// The tokens as the catalogue reads them (§8.5): `tokens.css`, and an app's
// theme, read as the platform reads them (`src/apps/theme/tokens.rs`) — the
// light block, then the dark block — so a frame can show light and dark
// side by side whatever the device is set to, and every token can be listed
// by what it is for.

const DARK = "@media (prefers-color-scheme: dark)";
const DECL = /--yen-([a-z0-9-]+)\s*:\s*([^;]+);/g;

/** Every `--yen-name: value;` in `css`, in order, as `[name, value]`. */
function declarations(css) {
  return [...css.matchAll(DECL)].map((m) => [m[1], m[2].trim()]);
}

/** A stylesheet's tokens: `{light, dark}`, each `[name, value]` in order. */
export function readTokens(css) {
  const text = css.replace(/\/\*[\s\S]*?\*\//g, "");
  const at = text.indexOf(DARK);
  const light = at === -1 ? text : text.slice(0, at);
  const dark = at === -1 ? "" : text.slice(at + DARK.length).split("@media")[0];
  return { light: declarations(light), dark: declarations(dark) };
}

const SURFACES = ["warm", "plain", "cool", "rose", "sage"];
const PALETTE = ["coral", "rose", "amber", "sage", "teal", "sky", "violet", "slate"];

/** What a token is for: the catalogue's section it is shown in. */
export function groupOf(name) {
  const first = name.split("-")[0];
  if (SURFACES.includes(first) && name !== first) return "surfaces";
  if (PALETTE.includes(name) || first === "chart") return "palette";
  if (["font", "text", "weight", "leading", "measure"].includes(first)) return "type";
  if (["space", "density", "control", "tap"].includes(first)) return "space";
  if (first === "radius") return "radius";
  if (first === "shadow") return "shadow";
  if (first === "duration" || first === "ease") return "motion";
  return "colour";
}

/**
 * The declarations that make an element show `scheme` — light or dark —
 * whatever the device is set to, with `theme`'s on top: every token
 * redeclared on it, so the names that point at others resolve there.
 */
export function framed(tokens, theme, scheme) {
  const out = new Map();
  // Motion is no scheme's: left to the page, it follows the device's "reduce motion".
  const put = (pairs) => pairs.filter(([n]) => groupOf(n) !== "motion").forEach(([n, v]) => out.set(n, v));
  put(tokens.light);
  if (scheme === "dark") put(tokens.dark);
  put(theme.light);
  if (scheme === "dark") put(theme.dark);
  return [...out];
}
