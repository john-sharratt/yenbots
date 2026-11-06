// What a page keeps in the browser between visits — its queued writes and
// what each collection last loaded — in IndexedDB, one database per app.
// A connection the browser closes — Safari does, for a page left in the
// background — is opened again on the next use.

/** `{load(key), save(key, value)}` over the database `name`. */
export function openKeep(name) {
  let db = null;
  const connect = () =>
    (db ??= new Promise((resolve, reject) => {
      const req = indexedDB.open(name, 1);
      req.onupgradeneeded = () => req.result.createObjectStore("kept");
      req.onsuccess = () => {
        const d = req.result;
        d.onclose = () => {
          db = null;
        };
        d.onversionchange = () => {
          d.close();
          db = null;
        };
        resolve(d);
      };
      req.onerror = () => {
        db = null;
        reject(req.error);
      };
      req.onblocked = () => {
        db = null;
        reject(new Error("another tab holds the browser's store open"));
      };
    }));
  const once = (mode, work) =>
    connect().then(
      (d) =>
        new Promise((resolve, reject) => {
          const tx = d.transaction("kept", mode);
          const req = work(tx.objectStore("kept"));
          tx.oncomplete = () => resolve(req.result);
          tx.onerror = () => reject(tx.error);
          tx.onabort = () => reject(tx.error);
        }),
    );
  // A connection closed under it fails the transaction: once more, on a new one.
  const run = (mode, work) =>
    once(mode, work).catch(() => {
      db = null;
      return once(mode, work);
    });
  return {
    load: (key) => run("readonly", (s) => s.get(key)),
    save: (key, value) => run("readwrite", (s) => s.put(value, key)),
  };
}

/** One key of a keep, as a `{load, save}` of its own. */
export function keyOf(keep, key) {
  return {
    load: () => keep.load(key),
    save: (value) => keep.save(key, value),
  };
}
