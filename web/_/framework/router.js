// An app's pages (§8.2): each a route, a pattern relative to the app's
// address — "" for its first page, "r/:id" for one recipe — with the page
// component shown for it. The first route that fits the address wins.

/** The routes of `defineApp({routes})`, in the order they were written. */
export function routesOf(spec) {
  return Object.entries(spec ?? {});
}

/** The address's path inside the app at `base` (`/a/recipes/`), or null for one outside it. */
export function within(base, pathname) {
  if (pathname.startsWith(base)) return pathname.slice(base.length);
  if (pathname + "/" === base) return "";
  return null;
}

function segments(path) {
  return path.split("/").filter((s) => s !== "");
}

function decoded(segment) {
  try {
    return decodeURIComponent(segment);
  } catch {
    return segment;
  }
}

/** The route `path` shows, with its parameters, or null when none fits. */
export function match(routes, path) {
  const given = segments(path).map(decoded);
  for (const [pattern, page] of routes) {
    const parts = segments(pattern);
    if (parts.length !== given.length) continue;
    const params = {};
    let fits = true;
    for (let i = 0; i < parts.length && fits; i++) {
      if (parts[i].startsWith(":")) params[parts[i].slice(1)] = given[i];
      else fits = parts[i] === given[i];
    }
    if (fits) return { pattern, page, params };
  }
  return null;
}

/** The app id an address names: `/a/recipes/r/7` is `recipes`. */
export function appIdOf(pathname) {
  const m = /^\/a\/([a-z][a-z0-9_-]*)(\/|$)/.exec(pathname);
  return m ? m[1] : null;
}
