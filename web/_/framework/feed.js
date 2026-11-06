// The site's two event streams a page listens to (§5.3, §6.1): the app's
// change feed, which keeps its collections current, and the reload signal,
// which brings a page onto the commit the site now serves.
//
// The browser reconnects a stream on its own after a dropped connection,
// but gives one up for good when an answer is an error — the gateway's,
// while the site restarts — so a stream it closed is opened again after
// `REOPEN_MS`.

/** How long a stream the browser gave up on waits before it is opened again. */
export const REOPEN_MS = 5_000;

/** `EventSource.CLOSED`. */
const CLOSED = 2;

/**
 * A stream at `url`, kept open: `wire(source)` adds the listeners to each
 * source made, and `lost()` is told each time the connection went.
 */
function kept(url, EventSource, later, wire, lost) {
  const open = () => {
    const source = new EventSource(url);
    wire(source);
    source.addEventListener("error", () => {
      lost();
      if (source.readyState === CLOSED) later(open, REOPEN_MS);
    });
    return source;
  };
  return open();
}

/**
 * Tell `heard` each change on the app's feed at `url`, and `{reset: true}`
 * when the page should read everything again: on a restore, and on coming
 * back after the connection went, when changes may have been missed.
 */
export function listenForChanges(url, EventSource, heard, later = setTimeout) {
  let lost = false;
  return kept(
    url,
    EventSource,
    later,
    (source) => {
      source.addEventListener("change", (e) => heard(JSON.parse(e.data)));
      source.addEventListener("reset", () => heard({ reset: true }));
      source.addEventListener("open", () => {
        if (lost) {
          lost = false;
          heard({ reset: true });
        }
      });
    },
    () => {
      lost = true;
    },
  );
}

/**
 * Reload the page each time the site serves a new commit — at once when it
 * is showing, or when it next shows, so a page hidden when an edit landed
 * never comes back as the old one.
 */
export function listenForReload(EventSource, page, reload, later = setTimeout) {
  let owed = false;
  page.addEventListener("visibilitychange", () => {
    if (owed && page.visibilityState === "visible") {
      owed = false;
      reload();
    }
  });
  return kept(
    "/api/v1/events/_reload",
    EventSource,
    later,
    (source) =>
      source.addEventListener("reload", () => {
        if (page.visibilityState === "visible") reload();
        else owed = true;
      }),
    () => {},
  );
}
