// A month as a calendar shows it: whole weeks, Monday first (en-AU), each
// day `{date, day, inMonth}` with `date` as `YYYY-MM-DD` — the form a `date`
// field holds (§7.5). Months count from 1.

const pad = (n) => String(n).padStart(2, "0");

/** `YYYY-MM-DD` for a year, a month from 1, and a day. */
export function isoDate(year, month, day) {
  return `${String(year).padStart(4, "0")}-${pad(month)}-${pad(day)}`;
}

/** The year, month (from 1) and day a `YYYY-MM-DD` names. */
export function parts(date) {
  const [y, m, d] = date.split("-").map(Number);
  return { year: y, month: m, day: d };
}

/** How many days `month` (from 1) of `year` has. */
export function daysIn(year, month) {
  return new Date(Date.UTC(year, month, 0)).getUTCDate();
}

/** The weekday of a date, Monday 0 to Sunday 6. */
export function weekday(year, month, day) {
  return (new Date(Date.UTC(year, month - 1, day)).getUTCDay() + 6) % 7;
}

/** The month `by` months after `month` of `year`: `{year, month}`. */
export function shift(year, month, by) {
  const at = year * 12 + (month - 1) + by;
  return { year: Math.floor(at / 12), month: (at % 12 + 12) % 12 + 1 };
}

/** The date `n` days after `date`, both `YYYY-MM-DD`. */
export function addDays(date, n) {
  const { year, month, day } = parts(date);
  const d = new Date(Date.UTC(year, month - 1, day + n));
  return isoDate(d.getUTCFullYear(), d.getUTCMonth() + 1, d.getUTCDate());
}

/**
 * The date a key moves a calendar's focus to from `date`, as the ARIA date
 * picker does: a day with the arrows left and right, a week up and down,
 * the week's ends with Home and End, a month with Page Up and Page Down (the
 * same day, or the month's last when it has fewer). Null for any other key.
 */
export function calendarMove(date, key) {
  const { year, month, day } = parts(date);
  const wd = weekday(year, month, day);
  const monthAway = (by) => {
    const to = shift(year, month, by);
    return isoDate(to.year, to.month, Math.min(day, daysIn(to.year, to.month)));
  };
  switch (key) {
    case "ArrowRight":
      return addDays(date, 1);
    case "ArrowLeft":
      return addDays(date, -1);
    case "ArrowDown":
      return addDays(date, 7);
    case "ArrowUp":
      return addDays(date, -7);
    case "Home":
      return addDays(date, -wd);
    case "End":
      return addDays(date, 6 - wd);
    case "PageDown":
      return monthAway(1);
    case "PageUp":
      return monthAway(-1);
    default:
      return null;
  }
}

/** The weeks that show `month` of `year`, each seven days, Monday first. */
export function weeksOf(year, month) {
  const lead = weekday(year, month, 1);
  const before = shift(year, month, -1);
  const after = shift(year, month, 1);
  const inBefore = daysIn(before.year, before.month);
  const days = [];
  for (let i = lead; i > 0; i--) {
    const d = inBefore - i + 1;
    days.push({ date: isoDate(before.year, before.month, d), day: d, inMonth: false });
  }
  for (let d = 1; d <= daysIn(year, month); d++) {
    days.push({ date: isoDate(year, month, d), day: d, inMonth: true });
  }
  for (let d = 1; days.length % 7 !== 0; d++) {
    days.push({ date: isoDate(after.year, after.month, d), day: d, inMonth: false });
  }
  const weeks = [];
  for (let i = 0; i < days.length; i += 7) weeks.push(days.slice(i, i + 7));
  return weeks;
}
