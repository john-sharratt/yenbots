// The site's service worker (§11.1), at `/sw.js` so it may handle every
// page's pushes. It shows each push the site sends — what the inbox already
// holds — and a tap marks it read and opens the page it names, or brings
// that page forward if it is open. Every open page is told a notification
// arrived, so its bell updates at once.

self.addEventListener("install", () => self.skipWaiting());
self.addEventListener("activate", (e) => e.waitUntil(self.clients.claim()));

self.addEventListener("push", (e) => {
  let shown = {};
  try {
    shown = e.data ? e.data.json() : {};
  } catch {
    shown = { title: e.data ? e.data.text() : "" };
  }
  const title = shown.title || "Yen Bot";
  const telling = self.clients
    .matchAll({ type: "window", includeUncontrolled: true })
    .then((open) => open.forEach((c) => c.postMessage({ type: "yen-notification", id: shown.id })));
  e.waitUntil(
    Promise.all([
      self.registration.showNotification(title, {
        body: shown.body || "",
        icon: shown.icon || "/_library/tiles/heart-coral-256.png",
        badge: "/_library/tiles/heart-coral-128.png",
        tag: shown.id,
        data: { open: shown.open || "/", id: shown.id },
      }),
      telling,
    ]),
  );
});

self.addEventListener("notificationclick", (e) => {
  e.notification.close();
  const open = new URL(e.notification.data?.open || "/", self.location.origin).href;
  const id = e.notification.data?.id;
  const read = id
    ? fetch(`/api/v1/inbox/${encodeURIComponent(id)}/read`, {
        method: "POST",
        headers: { "X-Yen-App": "_site" },
        credentials: "same-origin",
      }).catch(() => {})
    : Promise.resolve();
  e.waitUntil(
    Promise.all([
      read,
      self.clients.matchAll({ type: "window", includeUncontrolled: true }).then((windows) => {
        const showing = windows.find((c) => c.url === open);
        return showing ? showing.focus() : self.clients.openWindow(open);
      }),
    ]),
  );
});
