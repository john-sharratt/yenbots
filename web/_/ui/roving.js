// Moving between a group's items with the keyboard — tabs, a calendar's
// days, a photo grid — as the ARIA practices describe: the arrows step and
// wrap, Home and End go to the ends.

/**
 * The item `key` moves to from `at`, of `count` items laid `across` to a row
 * (1 for a single row of tabs read as a line), or null when the key is not
 * one that moves. Up and down move a row; in a single line they step.
 */
export function moveTo(at, key, count, across = count) {
  if (count <= 0) return null;
  const wrap = (i) => ((i % count) + count) % count;
  switch (key) {
    case "ArrowRight":
      return wrap(at + 1);
    case "ArrowLeft":
      return wrap(at - 1);
    case "ArrowDown":
      return across >= count ? wrap(at + 1) : Math.min(at + across, count - 1);
    case "ArrowUp":
      return across >= count ? wrap(at - 1) : Math.max(at - across, 0);
    case "Home":
      return 0;
    case "End":
      return count - 1;
    default:
      return null;
  }
}
