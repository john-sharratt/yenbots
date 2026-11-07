// What the admin page works out from the API's answers, kept apart from the
// page so it is tested on its own: each app's use over the period, and how
// a time is said.

/**
 * Each app's use over the period, most opened first: opens and AI calls
 * summed from `byDay`, and the accounts that opened it counted from
 * `byAccount` (`GET /admin/usage`'s `usage` by `day` and by `account`).
 */
export function usageByApp(byDay, byAccount) {
  const apps = new Map();
  const of = (app) => {
    if (!apps.has(app)) apps.set(app, { app, opens: 0, accounts: 0, chats: 0, pictures: 0, questions: 0, days: 0 });
    return apps.get(app);
  };
  for (const row of byDay) {
    const a = of(row.app);
    a.opens += row.opens;
    a.chats += row.chats;
    a.pictures += row.pictures;
    a.questions += row.questions;
    if (row.opens > 0) a.days += 1;
  }
  for (const row of byAccount) {
    if (row.opens > 0 && row.key !== "") of(row.app).accounts += 1;
  }
  return [...apps.values()].sort((x, y) => y.opens - x.opens || (x.app < y.app ? -1 : 1));
}

/** `ms` as a short date and time in the browser's language, or "never". */
export function when(ms, locale = "en-AU") {
  if (ms === null || ms === undefined) return "never";
  return new Date(ms).toLocaleString(locale, { day: "numeric", month: "short", hour: "numeric", minute: "2-digit" });
}

/** A number of bytes as people read it. */
export function size(bytes) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

/** The order after moving app `id` one place `by` (−1 up, +1 down). */
export function moved(order, id, by) {
  const at = order.indexOf(id);
  const to = at + by;
  if (at < 0 || to < 0 || to >= order.length) return order;
  const next = [...order];
  [next[at], next[to]] = [next[to], next[at]];
  return next;
}
