// The landing page: who the visitor is, and what they can do next. It
// reloads itself whenever the site starts serving a new commit.
new EventSource("/api/v1/events/_reload").addEventListener("reload", () => location.reload());

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
  who.textContent = "This account can't use Yen Bots.";
} else {
  who.textContent = `Welcome, ${me.account.name}. Apps arrive here soon.`;
}
