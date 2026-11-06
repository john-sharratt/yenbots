// Every Vietnamese letter (§8.7), composed: the twelve vowels each with the
// five tones and none, then d and đ — the set the faces must draw, as the
// test page shows it.

const VOWELS = ["a", "ă", "â", "e", "ê", "i", "o", "ô", "ơ", "u", "ư", "y"];
const TONES = ["", "̀", "́", "̉", "̃", "̣"];

/** The letters in rows — a vowel's six forms, then d and đ — lowercase or capital. */
export function letters(upper) {
  const rows = VOWELS.map((v) => TONES.map((t) => (v + t).normalize("NFC")));
  rows.push(["d", "đ"]);
  return upper ? rows.map((r) => r.map((l) => l.toUpperCase())) : rows;
}
