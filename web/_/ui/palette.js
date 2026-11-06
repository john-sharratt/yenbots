// The palette's colours by name (`tokens.css`, §15): the colours a tile, an
// illustration or a chart series may be, and the one a name is given when
// it needs a colour of its own — an avatar with no picture — the same for
// the same name on every page.

export const PALETTE = ["coral", "rose", "amber", "sage", "teal", "sky", "violet", "slate"];

/** The palette colour `name` is always given. */
export function paletteOf(name) {
  let sum = 0;
  for (const ch of String(name ?? "").normalize("NFC")) sum += ch.codePointAt(0);
  return PALETTE[sum % PALETTE.length];
}
