// Days as the site keeps them, YYYY-MM-DD, worked out in the device's own time.

const SHOW = new Intl.DateTimeFormat("en-AU", { weekday: "short", day: "numeric", month: "short", year: "numeric" });

function iso(d) {
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

function parse(day) {
  const [y, m, d] = day.split("-").map(Number);
  return new Date(y, m - 1, d);
}

/** Today, here. */
export function today() {
  return iso(new Date());
}

/** The day `n` days after `day` (before it when negative). */
export function addDays(day, n) {
  const d = parse(day);
  d.setDate(d.getDate() + n);
  return iso(d);
}

/** The day `n` years after `day`. */
export function addYears(day, n) {
  const d = parse(day);
  d.setFullYear(d.getFullYear() + n);
  return iso(d);
}

/** The next anniversary of `start` from `now` on: a year from now when there is no start. */
export function nextAnniversary(start, now) {
  if (!start) return addYears(now, 1);
  let day = start;
  while (day <= now) day = addYears(day, 1);
  return day;
}

/** How many days from `a` to `b`. */
export function daysFrom(a, b) {
  const x = parse(a);
  const y = parse(b);
  return Math.round((Date.UTC(y.getFullYear(), y.getMonth(), y.getDate()) - Date.UTC(x.getFullYear(), x.getMonth(), x.getDate())) / 86400000);
}

/** `Tue 3 Dec 2026`. */
export function showDay(day) {
  return day ? SHOW.format(parse(day)) : "";
}

/** `today`, `in 3 days`, `2 days ago`. */
export function fromNow(day) {
  if (!day) return "";
  const n = daysFrom(today(), day);
  if (n === 0) return "today";
  if (n === 1) return "tomorrow";
  if (n === -1) return "yesterday";
  return n > 0 ? `in ${n} days` : `${-n} days ago`;
}
