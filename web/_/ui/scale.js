// A chart's arithmetic (§8.5): round tick values for an axis, and the
// shapes of bars and lines inside a box. Pure, so `<yen-chart>` only draws
// what these answer.

/** A round step near `span / count`: 1, 2 or 5 times a power of ten. */
export function niceStep(span, count) {
  if (!(span > 0)) return 1;
  const raw = span / Math.max(1, count);
  const power = 10 ** Math.floor(Math.log10(raw));
  const f = raw / power;
  const nice = f <= 1 ? 1 : f <= 2 ? 2 : f <= 5 ? 5 : 10;
  return nice * power;
}

/** Round the float noise out of a tick: `0.30000000000000004` is `0.3`. */
const tidy = (x) => Number(x.toPrecision(12));

/**
 * About `count` round ticks covering `min..max`, from zero when every value
 * is above it (a bar starts at zero), each a multiple of one step.
 */
export function ticks(min, max, count = 5) {
  let lo = Math.min(0, min);
  let hi = Math.max(0, max);
  if (lo === hi) hi = lo + 1;
  const step = niceStep(hi - lo, count);
  lo = Math.floor(lo / step) * step;
  hi = Math.ceil(hi / step) * step;
  const out = [];
  for (let v = lo; v <= hi + step / 2; v += step) out.push(tidy(v));
  return out;
}

/** A linear map from `[d0, d1]` onto `[r0, r1]`. */
export function linear([d0, d1], [r0, r1]) {
  const span = d1 - d0 || 1;
  return (v) => r0 + ((v - d0) / span) * (r1 - r0);
}

/**
 * One bar per value, in a box `width` by `height`, with `gap` between bars
 * as a fraction of each slot: `{x, y, width, height}`, y downwards, every
 * number to two places. Negative values hang below the zero line.
 */
export function bars(values, { width, height, gap = 0.25, axis }) {
  const t = axis ?? ticks(Math.min(...values, 0), Math.max(...values, 0));
  const y = linear([t[0], t[t.length - 1]], [height, 0]);
  const slot = width / Math.max(1, values.length);
  const w = slot * (1 - gap);
  const zero = y(0);
  const r = (n) => Math.round(n * 100) / 100;
  return values.map((v, i) => {
    const top = Math.min(y(v), zero);
    return { x: r(i * slot + (slot - w) / 2), y: r(top), width: r(w), height: r(Math.abs(zero - y(v))) };
  });
}

/** An SVG path through `values`, evenly spaced across `width`, on `axis`'s scale. */
export function line(values, { width, height, axis }) {
  const t = axis ?? ticks(Math.min(...values), Math.max(...values));
  const y = linear([t[0], t[t.length - 1]], [height, 0]);
  const step = values.length > 1 ? width / (values.length - 1) : 0;
  const r = (n) => Math.round(n * 100) / 100;
  return values.map((v, i) => `${i === 0 ? "M" : "L"}${r(i * step)} ${r(y(v))}`).join(" ");
}
