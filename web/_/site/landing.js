// The landing page (§13.1): who the visitor is, and what they can do next;
// once they are in, their apps as tiles in the order Yen set, the bell, and
// for an admin the way to the admin page; and, on an iPhone or iPad not yet
// opened from its home screen, the walk-through to put it there, shown until
// it is put away (§11.1). It reloads itself whenever the site starts serving
// a new commit.
import { app } from "/_/yen.js";

new EventSource("/api/v1/events/_reload").addEventListener("reload", () => location.reload());

/** Where the walk-through being put away is remembered, on this device. */
const PUT_AWAY = "yenbot-home-screen-shown";

const who = document.getElementById("who");
const res = await fetch("/api/v1/me", { headers: { "X-Yen-App": "_site" } });
const me = await res.json();

who.textContent = "";
if (!me.signed_in) {
  const next = encodeURIComponent(location.href);
  const a = document.createElement("a");
  a.className = "button";
  a.textContent = "Sign in with Google";
  a.href = `/auth/login?next=${next}`;
  who.append(a);
} else if (me.account.status === "pending") {
  who.textContent = `Hi ${me.account.name} — Yen has been told you're here, and will let you in soon.`;
} else if (me.account.status === "blocked") {
  who.textContent = "This account can't use Yen Bot.";
} else {
  const bar = document.getElementById("bar");
  if (me.account.admin) {
    const admin = document.createElement("a");
    admin.className = "admin-link";
    admin.href = "/_admin";
    admin.textContent = "Admin";
    bar.append(admin);
  }
  bar.append(document.createElement("yen-bell"));
  await tiles(me.account.name);
  await walkThrough();
}

/** The account's apps, as tiles in the landing page's order. */
async function tiles(name) {
  const res = await fetch("/api/v1/apps", { headers: { "X-Yen-App": "_site" } });
  const apps = res.ok ? (await res.json()).apps : [];
  who.textContent =
    apps.length === 0 ? `Welcome, ${name}. Your apps arrive here soon.` : `Welcome, ${name}.`;
  const list = document.getElementById("tiles");
  list.textContent = "";
  for (const a of apps) {
    const li = document.createElement("li");
    const link = document.createElement("a");
    link.href = `/a/${a.id}/`;
    link.className = "tile";
    if (a.accent) link.style.setProperty("--tile-accent", a.accent);
    const art = document.createElement("span");
    art.className = "art";
    if (a.tile) {
      const img = document.createElement("img");
      img.src = a.tile;
      img.alt = "";
      img.width = 128;
      img.height = 128;
      art.append(img);
    } else {
      art.textContent = a.name.slice(0, 1).toUpperCase();
    }
    const label = document.createElement("span");
    label.className = "name";
    label.textContent = a.name;
    link.append(art, label);
    if (a.status === "draft") {
      const draft = document.createElement("span");
      draft.className = "draft";
      draft.textContent = "Draft";
      link.append(draft);
    }
    // The tile the page opens from carries over to it (§13.1).
    link.addEventListener("click", () => (art.style.viewTransitionName = "opening-tile"));
    li.append(link);
    list.append(li);
  }
  list.hidden = apps.length === 0;
}

/** On an iPhone or iPad in Safari, how to put the site on the home screen — once. */
async function walkThrough() {
  let seen = false;
  try {
    seen = localStorage.getItem(PUT_AWAY) === "1";
  } catch {
    // A browser that keeps nothing shows it each time.
  }
  if (seen || (await app.notify.check()) !== "install") return;
  const card = document.getElementById("home-screen");
  card.hidden = false;
  card.querySelector("button").addEventListener("click", () => {
    card.hidden = true;
    try {
      localStorage.setItem(PUT_AWAY, "1");
    } catch {
      // Nowhere to remember it; it is put away for this visit.
    }
  });
}
