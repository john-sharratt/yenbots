import { letters } from "../../web/_design/letters.js";
import { framed, groupOf, readTokens } from "../../web/_design/tokens.js";
import { eq, test } from "./harness.js";

test("the test page shows every Vietnamese letter, each one composed character", () => {
  const lower = letters(false).flat();
  const upper = letters(true).flat();
  eq([lower.length, upper.length], [74, 74]);
  eq(letters(false)[1], ["ă", "ằ", "ắ", "ẳ", "ẵ", "ặ"]);
  eq(letters(true)[10], ["Ư", "Ừ", "Ứ", "Ử", "Ữ", "Ự"]);
  eq(letters(true)[12], ["D", "Đ"]);
  eq(
    [...lower, ...upper].filter((l) => [...l].length !== 1),
    [],
    "no letter needs a combining mark",
  );
});

const CSS = `/* the house: --yen-gone: #000; */
:root {
  --yen-bg: var(--yen-warm-bg);
  --yen-warm-bg: #fbf7f2; /* why */
  --yen-accent: #c2375b;
}
@media (prefers-color-scheme: dark) {
  :root {
    --yen-warm-bg: #1a1613;
    --yen-accent: #f27b98;
  }
}
@media (prefers-reduced-motion: reduce) {
  :root { --yen-duration-1: 0.01ms; }
}`;

test("the tokens are read by mode, as the platform reads them", () => {
  eq(readTokens(CSS), {
    light: [
      ["bg", "var(--yen-warm-bg)"],
      ["warm-bg", "#fbf7f2"],
      ["accent", "#c2375b"],
    ],
    dark: [
      ["warm-bg", "#1a1613"],
      ["accent", "#f27b98"],
    ],
  });
  eq(readTokens(":root { --yen-a: 1; }"), { light: [["a", "1"]], dark: [] });
});

test("a frame redeclares every token, then the theme's, for its scheme", () => {
  const tokens = readTokens(CSS);
  const theme = readTokens(":root {\n  --yen-accent: #b03a5b;\n}\n@media (prefers-color-scheme: dark) {\n  :root {\n    --yen-accent: #f07a95;\n  }\n}\n");
  eq(framed(tokens, theme, "light"), [
    ["bg", "var(--yen-warm-bg)"],
    ["warm-bg", "#fbf7f2"],
    ["accent", "#b03a5b"],
  ]);
  eq(framed(tokens, theme, "dark"), [
    ["bg", "var(--yen-warm-bg)"],
    ["warm-bg", "#1a1613"],
    ["accent", "#f07a95"],
  ]);
  eq(framed(tokens, { light: [], dark: [] }, "dark")[2], ["accent", "#f27b98"]);
  const moving = readTokens(":root {\n  --yen-duration-1: 120ms;\n  --yen-ease-out: ease;\n  --yen-bg: #fff;\n}\n");
  eq(framed(moving, { light: [], dark: [] }, "light"), [["bg", "#fff"]], "motion is left to the page");
});

test("each token is shown with what it is for", () => {
  eq(
    ["warm-bg", "rose-line-strong", "rose", "chart-2", "bg", "ink-2", "accent-soft", "on-overlay"].map(groupOf),
    ["surfaces", "surfaces", "palette", "palette", "colour", "colour", "colour", "colour"],
  );
  eq(
    ["font-heading", "text-xl", "weight-bold", "leading", "measure", "space-4", "density-roomy", "control-height", "tap"].map(groupOf),
    ["type", "type", "type", "type", "type", "space", "space", "space", "space"],
  );
  eq(["radius-round-1", "shadow-2", "duration-1", "ease-out"].map(groupOf), ["radius", "shadow", "motion", "motion"]);
});
