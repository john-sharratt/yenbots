// An account's initials, for an avatar with no picture: the first letter of
// the first word and of the last — "Mary Jane Watson" is "MW", "Yen" is "Y".
// A letter keeps its accent (`É`), uppercased; a name with no letters has
// none.

const LETTER = /\p{L}\p{M}*/u;

export function initials(name) {
  const words = String(name ?? "")
    .normalize("NFC")
    .split(/\s+/u)
    .map((w) => w.match(LETTER)?.[0])
    .filter(Boolean);
  if (words.length === 0) return "";
  const first = words[0];
  const last = words.length > 1 ? words[words.length - 1] : "";
  return (first + last).toUpperCase();
}
