// The framework's tests, run in QuickJS by `cargo test`
// (`src/framework_tests.rs`). A test file registers its tests with `test`;
// the runner runs every one and reports each by name.

const tests = [];

export function test(name, fn) {
  tests.push({ name, fn });
}

/** `value` as JSON with every object's keys sorted, so equal values print equal. */
function canon(value) {
  if (Array.isArray(value)) return `[${value.map(canon).join(",")}]`;
  if (value && typeof value === "object") {
    const keys = Object.keys(value).sort();
    return `{${keys.map((k) => `${JSON.stringify(k)}:${canon(value[k])}`).join(",")}}`;
  }
  return value === undefined ? "undefined" : JSON.stringify(value);
}

export function eq(actual, expected, what = "") {
  const a = canon(actual);
  const e = canon(expected);
  if (a !== e) throw new Error(`${what ? `${what}: ` : ""}expected ${e}, got ${a}`);
}

/** The error `promise` rejects with; fails when it fulfils. */
export async function rejection(promise) {
  try {
    await promise;
  } catch (e) {
    return e;
  }
  throw new Error("expected a rejection");
}

/** Let every job already queued run. */
export async function settle() {
  for (let i = 0; i < 20; i++) await Promise.resolve();
}

export async function run() {
  const out = [];
  for (const t of tests) {
    try {
      await t.fn();
      out.push({ name: t.name, ok: true });
    } catch (e) {
      out.push({ name: t.name, ok: false, error: `${e}\n${e?.stack ?? ""}` });
    }
  }
  return out;
}
