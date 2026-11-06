import { accepted, refusal, sizeOf } from "/_/ui/accepts.js";
import { initials } from "/_/ui/initials.js";
import { addDays, calendarMove, daysIn, isoDate, parts, shift, weekday, weeksOf } from "/_/ui/month.js";
import { PALETTE, paletteOf } from "/_/ui/palette.js";
import { moveTo } from "/_/ui/roving.js";
import { bars, line, niceStep, ticks } from "/_/ui/scale.js";
import { MOST, Toasts } from "/_/ui/toasts.js";
import { eq, test } from "./harness.js";

test("initials are the first and last words' first letters, accents kept", () => {
  eq(initials("Mary Jane Watson"), "MW");
  eq(initials("yen"), "Y");
  eq(initials("  élodie   de  la cruz "), "ÉC");
  eq(initials("émile zola"), "ÉZ", "written decomposed, answered composed");
  eq(initials(""), "");
  eq(initials(null), "");
  eq(initials("123 !"), "", "no letters, no initials");
});

test("a name is given the same palette colour on every page", () => {
  eq(PALETTE.length, 8);
  eq([paletteOf("Yen"), paletteOf(""), paletteOf(null)], ["teal", "coral", "coral"]);
  eq(paletteOf("Zoë"), paletteOf("Zoë"), "decomposed or composed, one colour");
});

test("a month is whole weeks, Monday first", () => {
  const nov = weeksOf(2026, 11);
  eq(nov.length, 6);
  eq(
    nov[0].map((d) => [d.date, d.inMonth]),
    [
      ["2026-10-26", false],
      ["2026-10-27", false],
      ["2026-10-28", false],
      ["2026-10-29", false],
      ["2026-10-30", false],
      ["2026-10-31", false],
      ["2026-11-01", true],
    ],
  );
  eq(
    nov[5].map((d) => d.day),
    [30, 1, 2, 3, 4, 5, 6],
  );
  eq(nov[5][0].inMonth, true);
  const feb = weeksOf(2021, 2);
  eq([feb.length, feb[0][0].date, feb[3][6].date], [4, "2021-02-01", "2021-02-28"], "a month that starts on a Monday");
  eq([weekday(2026, 11, 7), weekday(2026, 11, 1)], [5, 6], "Saturday, Sunday");
});

test("days, months and dates", () => {
  eq([daysIn(2024, 2), daysIn(2100, 2), daysIn(2000, 2), daysIn(2026, 4)], [29, 28, 29, 30]);
  eq(shift(2026, 12, 1), { year: 2027, month: 1 });
  eq(shift(2026, 1, -1), { year: 2025, month: 12 });
  eq(shift(2026, 11, -23), { year: 2024, month: 12 });
  eq(isoDate(2026, 3, 9), "2026-03-09");
  eq(parts("2026-03-09"), { year: 2026, month: 3, day: 9 });
  eq([addDays("2026-11-30", 1), addDays("2026-01-01", -1), addDays("2024-02-28", 1)], ["2026-12-01", "2025-12-31", "2024-02-29"]);
});

test("the keys move a calendar's day as a date picker's do", () => {
  const from = (key) => calendarMove("2026-11-07", key);
  eq(
    ["ArrowRight", "ArrowLeft", "ArrowDown", "ArrowUp", "Home", "End", "PageDown", "PageUp", "x"].map(from),
    ["2026-11-08", "2026-11-06", "2026-11-14", "2026-10-31", "2026-11-02", "2026-11-08", "2026-12-07", "2026-10-07", null],
  );
  eq(calendarMove("2026-01-31", "PageDown"), "2026-02-28", "to a shorter month, its last day");
  eq(calendarMove("2026-03-31", "PageUp"), "2026-02-28");
});

test("the arrows move through a line and a grid, wrapping only along a line", () => {
  eq([moveTo(0, "ArrowLeft", 3), moveTo(2, "ArrowRight", 3), moveTo(1, "ArrowDown", 3)], [2, 0, 2]);
  eq([moveTo(1, "Home", 3), moveTo(1, "End", 3), moveTo(1, "a", 3), moveTo(0, "End", 0)], [0, 2, null, null]);
  eq([moveTo(3, "ArrowDown", 35, 7), moveTo(32, "ArrowDown", 35, 7), moveTo(3, "ArrowUp", 35, 7)], [10, 34, 0]);
});

test("an axis has round ticks, from zero", () => {
  eq([niceStep(100, 5), niceStep(87, 5), niceStep(0, 5)], [20, 20, 1]);
  eq(ticks(0, 87), [0, 20, 40, 60, 80, 100]);
  eq(ticks(-3, 7), [-4, -2, 0, 2, 4, 6, 8]);
  eq(ticks(0.1, 0.3), [0, 0.1, 0.2, 0.3], "no float noise");
  eq(ticks(5, 5), [0, 1, 2, 3, 4, 5]);
  eq(ticks(0, 0), [0, 0.2, 0.4, 0.6, 0.8, 1]);
});

test("bars stand on zero, and hang below it", () => {
  eq(bars([10, 20, 40], { width: 300, height: 100 }), [
    { x: 12.5, y: 75, width: 75, height: 25 },
    { x: 112.5, y: 50, width: 75, height: 50 },
    { x: 212.5, y: 0, width: 75, height: 100 },
  ]);
  eq(bars([-5, 5], { width: 100, height: 100 }), [
    { x: 6.25, y: 50, width: 37.5, height: 41.67 },
    { x: 56.25, y: 8.33, width: 37.5, height: 41.67 },
  ]);
});

test("a line is a path through its values", () => {
  eq(line([0, 5, 10], { width: 200, height: 50 }), "M0 50 L100 25 L200 0");
  eq(line([3], { width: 200, height: 50 }), "M0 0", "one value, at the top of an axis from zero");
});

test("toasts go by themselves, an action stays, and the oldest plain one makes way", () => {
  const timers = [];
  const t = new Toasts((fn, ms) => timers.push([fn, ms]));
  const shown = () => t.shown.value.map((x) => x.id);
  eq(t.show("Saved"), 1);
  t.show("Deleted", { action: { label: "Undo", run: () => {} } });
  t.show("c");
  t.show("d");
  eq(shown(), [2, 3, 4], `at most ${MOST}`);
  t.show("e");
  eq(shown(), [2, 4, 5]);
  eq(
    timers.map(([, ms]) => ms),
    [4000, 4000, 4000, 4000],
    "the one with an action has no timer",
  );
  for (const [fn] of timers) fn();
  eq(shown(), [2]);
  t.dismiss(2);
  eq(shown(), []);
  const full = new Toasts(() => {});
  const act = { label: "Undo", run: () => {} };
  full.show("a", { action: act });
  full.show("b", { action: act });
  full.show("c", { action: act });
  full.show("d");
  eq(
    full.shown.value.map((x) => x.message),
    ["b", "c", "d"],
    "the newest is never the one that makes way",
  );
});

test("a picker takes the kinds it names, up to its size", () => {
  eq(accepted("image/*", { type: "image/png" }), true);
  eq(accepted("image/*", { type: "application/pdf" }), false);
  eq(accepted(".pdf, image/png", { name: "Menu.PDF", type: "" }), true);
  eq(accepted("", { name: "x", type: "text/plain" }), true);
  eq([sizeOf(999), sizeOf(820_400), sizeOf(3_400_000)], ["999 bytes", "820 KB", "3.4 MB"]);
  const big = { name: "a.png", type: "image/png", size: 6_000_000 };
  eq(refusal(big, { accept: "image/*", maxBytes: 5_000_000 }), "is 6.0 MB; the most is 5.0 MB");
  eq(refusal({ name: "a.txt", type: "text/plain", size: 1 }, { accept: "image/*" }), "isn't a kind of file this takes");
  eq(refusal({ ...big, size: 10 }, { accept: "image/*", maxBytes: 5_000_000 }), null);
});
