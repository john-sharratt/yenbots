// The landing page: who the visitor is, and what they can do next; the bell,
// once they are in; and, on an iPhone or iPad not yet opened from its home
// screen, the walk-through to put it there, shown until it is put away
// (§11.1). It reloads itself whenever the site starts serving a new commit.
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
  who.textContent = `Welcome, ${me.account.name}. Apps arrive here soon.`;
  document.getElementById("bar").append(document.createElement("yen-bell"));
  await walkThrough();
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
