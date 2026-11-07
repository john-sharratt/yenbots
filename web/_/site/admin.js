// The admin page (`/_admin`, §13.2): accounts and registration, groups,
// apps — visibility, shares, order, check, storage, export, archive — usage,
// jobs, each app's keys and hooks, errors and feedback, and the archive.
// Everything it does is a call to `/api/v1/admin/…` (`docs/api/`), so a Zen
// task can do the same; it writes nothing a route does not.

import { moved, size, usageByApp, when } from "./admin-summary.js";

const $ = (id) => document.getElementById(id);

/** An element: `tag`, attributes and listeners (`on<event>`), children. */
function h(tag, attrs = {}, ...children) {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (v === null || v === undefined || v === false) continue;
    if (k.startsWith("on")) el.addEventListener(k.slice(2), v);
    else if (k === "class") el.className = v;
    else el.setAttribute(k, v === true ? "" : String(v));
  }
  for (const c of children.flat()) {
    if (c === null || c === undefined || c === false) continue;
    el.append(c instanceof Node ? c : String(c));
  }
  return el;
}

/** Say `text` in the status line; `bad` for something that went wrong. */
function say(text, bad = false) {
  const s = $("status");
  s.textContent = text;
  s.classList.toggle("bad", bad);
}

class Refused extends Error {}

/** A call to the API as the page; a refusal throws with its detail. */
async function call(method, path, body) {
  const init = { method, headers: { "X-Yen-App": "_site" } };
  if (body !== undefined) {
    init.headers["Content-Type"] = "application/json";
    init.body = JSON.stringify(body);
  }
  const res = await fetch(`/api/v1${path}`, init);
  const answer = await res.json().catch(() => ({}));
  if (!res.ok) throw new Refused(answer.detail ?? `${method} ${path} answered ${res.status}`);
  return answer;
}

/** A file's text sent as an import's body (`POST /import/<app>`, §20.1). */
async function imported(app, file, coll) {
  const csv = file.name.toLowerCase().endsWith(".csv");
  const q = csv ? `?format=csv&coll=${encodeURIComponent(coll)}` : "";
  const res = await fetch(`/api/v1/import/${app}${q}`, {
    method: "POST",
    headers: { "X-Yen-App": "_site", "Content-Type": csv ? "text/csv" : "application/json" },
    body: await file.text(),
  });
  const answer = await res.json().catch(() => ({}));
  if (!res.ok) throw new Refused(answer.detail ?? `the import answered ${res.status}`);
  const made = Object.entries(answer.created ?? {}).map(([c, n]) => `${n} into ${c}`);
  return made.length ? `Imported ${made.join(", ")}.` : "The file held no records.";
}

/** Run `work`, then draw the page again; say what went wrong, if anything. */
async function act(work, done) {
  try {
    await work();
    if (done) say(done);
    await draw();
  } catch (e) {
    say(e.message, true);
  }
}

/** A table of `rows`, each a list of cells, under `heads`. */
function table(heads, rows) {
  if (rows.length === 0) return h("p", { class: "empty" }, "Nothing yet.");
  return h(
    "div",
    { class: "scroll" },
    h("table", {}, h("thead", {}, h("tr", {}, heads.map((t) => h("th", {}, t)))), h("tbody", {}, rows.map((r) => h("tr", {}, r)))),
  );
}

const cell = (...c) => h("td", {}, ...c);
const actions = (...c) => h("td", { class: "actions" }, ...c);
const button = (label, onclick, cls) => h("button", { type: "button", class: cls, onclick }, label);

let accounts = [];
const nameOf = (id) => accounts.find((a) => a.id === id)?.name ?? id;

function drawAccounts(answer) {
  accounts = answer.accounts;
  for (const r of document.querySelectorAll('input[name="registration"]')) r.checked = r.value === answer.registration;
  const pending = accounts.filter((a) => a.status === "pending");
  $("pending").replaceChildren(
    pending.length === 0
      ? h("p", { class: "empty" }, "No one is waiting.")
      : h("div", {}, h("h3", {}, "Waiting to be let in"), table(
          ["Name", "Address", "Asked"],
          pending.map((a) => [
            cell(a.name),
            cell(a.email),
            cell(when(a.created_ms)),
            actions(
              button("Approve", () => act(() => call("POST", `/admin/accounts/${a.id}/approve`), `${a.name} is in.`), "go"),
              button("Refuse", () => act(() => call("POST", `/admin/accounts/${a.id}/refuse`), `${a.name} was turned away.`), "danger"),
            ),
          ]),
        )),
  );
  const others = accounts.filter((a) => a.status !== "pending");
  $("everyone").replaceChildren(
    h("h3", {}, "Everyone"),
    table(
      ["Name", "Address", "Joined", "Last seen", "Groups", ""],
      others.map((a) => [
        cell(a.name, a.admin ? " (admin)" : ""),
        cell(a.email),
        cell(when(a.created_ms)),
        cell(when(a.seen_ms)),
        cell(groupsOf(a.id).join(", ")),
        actions(
          a.admin
            ? null
            : a.status === "blocked"
              ? button("Let back in", () => act(() => call("POST", `/admin/accounts/${a.id}/approve`), `${a.name} is back in.`))
              : button("Block", () => act(() => call("POST", `/admin/accounts/${a.id}/block`), `${a.name} is blocked.`), "danger"),
          a.admin
            ? null
            : button("Delete", () => {
                if (confirm(`Delete ${a.name}'s account? Their private records go with it; what they shared stays.`)) {
                  act(() => call("POST", `/admin/accounts/${a.id}/delete`), `${a.name}'s account is deleted.`);
                }
              }, "danger"),
        ),
      ]),
    ),
  );
}

let groups = [];
const groupsOf = (id) => groups.filter((g) => g.members.includes(id)).map((g) => g.name);

function drawGroups() {
  $("group-list").replaceChildren(
    ...(groups.length === 0
      ? [h("p", { class: "empty" }, "No groups yet.")]
      : groups.map((g) => {
          const outside = accounts.filter((a) => a.status === "active" && !g.members.includes(a.id));
          const pick = h("select", { "aria-label": `Add to ${g.name}` }, h("option", { value: "" }, "Add someone…"), outside.map((a) => h("option", { value: a.id }, a.name)));
          pick.addEventListener("change", () => {
            if (pick.value) act(() => call("POST", `/admin/groups/${g.name}/members`, { account: pick.value }), `Added to ${g.name}.`);
          });
          return h(
            "div",
            { class: "group" },
            h("h3", {}, g.name),
            h(
              "div",
              {},
              g.members.map((m) =>
                h("span", { class: "member" }, nameOf(m), button("✕", () => act(() => call("DELETE", `/admin/groups/${g.name}/members/${m}`), `Taken out of ${g.name}.`))),
              ),
            ),
            h("div", { class: "row" }, pick, button("Delete the group", () => {
              if (confirm(`Delete the group ${g.name}? Apps and records shared with it are shared with no one in its place.`)) {
                act(() => call("DELETE", `/admin/groups/${g.name}`), `${g.name} is deleted.`);
              }
            }, "danger")),
          );
        })),
  );
}

let apps = [];

/**
 * Import into app `a`: an export's JSON, or a spreadsheet's CSV into the
 * collection chosen — validated like any write, all or nothing.
 */
function importer(a) {
  const coll = h("select", { "aria-label": `The collection a CSV goes into, in ${a.name}` }, a.collections.map((c) => h("option", { value: c }, c)));
  const file = h("input", { type: "file", accept: ".csv,.json", "aria-label": `A file to import into ${a.name}` });
  file.addEventListener("change", () => {
    const chosen = file.files[0];
    if (chosen) act(async () => say(await imported(a.id, chosen, coll.value)));
    file.value = "";
  });
  return h("span", { class: "row" }, h("span", {}, "Import"), coll, file);
}

async function drawApps() {
  const order = apps.map((a) => a.id);
  const cards = await Promise.all(
    apps.map(async (a) => {
      const check = await call("GET", `/admin/apps/${a.id}/check`).catch(() => null);
      const art = a.tile ? h("img", { src: a.tile, alt: "" }) : h("span", { class: "initial" }, a.name.slice(0, 1));
      const shares = h("input", { value: a.access.shares.join(", "), "aria-label": `Who ${a.name} is shared with`, placeholder: "group:household, account:…" });
      const priv = h("input", { type: "checkbox", checked: a.access.private });
      priv.addEventListener("change", () =>
        act(() => call("PUT", `/admin/apps/${a.id}/visibility`, { private: priv.checked }), `${a.name} is ${priv.checked ? "private" : "public"}.`),
      );
      const refusals = check?.refusals ?? [];
      return h(
        "div",
        { class: "app" },
        art,
        h(
          "div",
          {},
          h("strong", {}, a.name), " ", h("code", {}, a.id), " · ", a.status,
          h("div", { class: "facts" }, `${a.storage.records} records · ${a.storage.files} files (${size(a.storage.file_bytes)}) · ${a.collections.length} collections`),
          refusals.length === 0
            ? h("div", { class: "facts" }, check?.migrating?.length ? `Migrating ${check.migrating.join(", ")}` : "Passes its check")
            : h("div", { class: "refused" }, `Refused: ${refusals.map((r) => `${r.path}${r.line ? `:${r.line}` : ""} ${r.message}`).join("; ")}`),
          h("label", { class: "row" }, h("span", {}, priv, " Private: only those it is shared with open it")),
          h(
            "div",
            { class: "row" },
            shares,
            button("Share", () => {
              const who = shares.value.split(",").map((s) => s.trim()).filter(Boolean);
              act(() => call("PUT", `/admin/apps/${a.id}/shares`, { who }), `${a.name}'s shares are saved.`);
            }),
          ),
          h(
            "div",
            { class: "row" },
            button("Up", () => act(() => call("PUT", "/admin/apps/order", { order: moved(order, a.id, -1) }))),
            button("Down", () => act(() => call("PUT", "/admin/apps/order", { order: moved(order, a.id, 1) }))),
            h("a", { href: `/api/v1/export/${a.id}`, download: `${a.id}.json` }, "Export"),
            importer(a),
            h("a", { href: `/a/${a.id}/` }, "Open"),
            button("Archive", () => {
              if (confirm(`Archive ${a.name}? It is hidden from everyone and its jobs held, until it is restored.`)) {
                act(() => call("POST", `/admin/apps/${a.id}/archive`), `${a.name} is archived.`);
              }
            }, "danger"),
          ),
        ),
      );
    }),
  );
  $("app-list").replaceChildren(...(cards.length ? cards : [h("p", { class: "empty" }, "No apps yet.")]));
}

async function drawUsage() {
  const [byDay, byAccount] = await Promise.all([call("GET", "/admin/usage?by=day"), call("GET", "/admin/usage?by=account")]);
  const rows = usageByApp(byDay.usage, byAccount.usage);
  $("usage-table").replaceChildren(
    table(
      ["App", "Opens", "People", "Days used", "Chats", "Pictures", "Questions"],
      rows.map((u) => [cell(u.app), cell(u.opens), cell(u.accounts), cell(u.days), cell(u.chats), cell(u.pictures), cell(u.questions)]),
    ),
  );
}

async function drawJobs() {
  const { jobs } = await call("GET", "/admin/jobs");
  $("job-list").replaceChildren(
    table(
      ["App", "Workflow", "State", "Next", "Last run", ""],
      jobs.map((j) => {
        const last = j.runs[0];
        return [
          cell(j.app),
          cell(j.workflow, j.rule ? ` (${j.rule})` : ""),
          cell(j.state),
          cell(when(j.next_ms)),
          cell(last ? `${when(last.started_ms)}: ${last.outcome}${last.error ? ` — ${last.error}` : ""}` : "never"),
          actions(
            button("Run now", () => act(() => call("POST", `/admin/jobs/${j.id}/run`), `${j.workflow} runs now.`)),
            j.state === "held"
              ? button("Release", () => act(() => call("POST", `/admin/jobs/${j.id}/release`)))
              : button("Hold", () => act(() => call("POST", `/admin/jobs/${j.id}/hold`))),
          ),
        ];
      }),
    ),
  );
}

let integrationApp = "";

async function drawIntegrations() {
  const pick = $("integration-app");
  if (!apps.some((a) => a.id === integrationApp)) integrationApp = apps[0]?.id ?? "";
  pick.replaceChildren(...apps.map((a) => h("option", { value: a.id, selected: a.id === integrationApp }, a.name)));
  if (!integrationApp) {
    $("secret-list").replaceChildren();
    $("hook-list").replaceChildren();
    return;
  }
  const app = integrationApp;
  const [{ secrets }, { hooks }] = await Promise.all([call("GET", `/admin/secrets/${app}`), call("GET", `/admin/hooks/${app}`)]);
  $("secret-list").replaceChildren(
    h("h3", {}, "Keys"),
    table(
      ["Name", "Set", ""],
      secrets.map((s) => [cell(h("code", {}, s.name)), cell(when(s.set_ms)), actions(button("Remove", () => act(() => call("DELETE", `/admin/secrets/${app}/${s.name}`), `${s.name} is removed.`), "danger"))]),
    ),
  );
  $("hook-list").replaceChildren(
    h("h3", {}, "Hooks"),
    table(
      ["Hook", "Address", "Token", ""],
      hooks.map((k) => [
        cell(h("code", {}, k.hook)),
        cell(h("code", {}, k.url)),
        cell(k.token_made_ms ? `made ${when(k.token_made_ms)}` : "none"),
        actions(
          button(k.token_made_ms ? "New token" : "Make a token", async () => {
            try {
              const made = await call("POST", `/admin/hooks/${app}/${k.hook}`);
              const shown = h("code", { class: "once" }, `${location.origin}${made.url}`);
              $("hook-list").append(h("p", {}, `The address for ${k.hook}, with its token — shown this once:`), shown);
              say(`A token for ${k.hook} is made: copy its address now.`);
            } catch (e) {
              say(e.message, true);
            }
          }),
          k.token_made_ms ? button("Revoke", () => act(() => call("DELETE", `/admin/hooks/${app}/${k.hook}`), `${k.hook}'s token is revoked.`), "danger") : null,
        ),
      ]),
    ),
  );
}

async function drawReports() {
  const [{ feedback }, { errors }] = await Promise.all([call("GET", "/admin/feedback?limit=50"), call("GET", "/admin/errors?limit=50")]);
  $("feedback-list").replaceChildren(table(["When", "App", "From", "Note"], feedback.map((n) => [cell(when(n.at_ms)), cell(n.app), cell(n.name ?? "a former member"), cell(n.text)])));
  $("error-list").replaceChildren(table(["When", "Where", "What"], errors.map((e) => [cell(when(e.at_ms)), cell(e.source), cell(e.message)])));
}

async function drawArchive() {
  const { archived } = await call("GET", "/admin/archive");
  $("archive-list").replaceChildren(
    table(
      ["App", "Why", "Since", "Purged", "Holds", ""],
      archived.map((a) => [
        cell(h("code", {}, a.app)),
        cell(a.reason === "removed" ? "its folder was removed" : "an admin archived it"),
        cell(when(a.archived_ms)),
        cell(a.purge_ms ? when(a.purge_ms) : "not while its folder is there"),
        cell(`${a.records} records, ${a.files} files`),
        actions(button("Restore", () => act(() => call("POST", `/admin/apps/${a.app}/restore`), `${a.app} is restored.`), "go")),
      ]),
    ),
  );
}

async function draw() {
  const [acc, grp, app] = await Promise.all([call("GET", "/admin/accounts"), call("GET", "/admin/groups"), call("GET", "/admin/apps")]);
  groups = grp.groups;
  drawAccounts(acc);
  drawGroups();
  apps = app.apps;
  await Promise.all([drawApps(), drawUsage(), drawJobs(), drawIntegrations(), drawReports(), drawArchive()]);
}

for (const r of document.querySelectorAll('input[name="registration"]')) {
  r.addEventListener("change", () => act(() => call("PUT", "/admin/registration", { registration: r.value }), r.value === "open" ? "New accounts are let in at once." : "New accounts wait for an admin."));
}
$("new-group").addEventListener("submit", (e) => {
  e.preventDefault();
  const name = e.target.elements.name.value.trim();
  act(() => call("POST", "/admin/groups", { name }), `${name} is made.`).then(() => e.target.reset());
});
$("integration-app").addEventListener("change", (e) => {
  integrationApp = e.target.value;
  drawIntegrations().catch((err) => say(err.message, true));
});
$("new-secret").addEventListener("submit", (e) => {
  e.preventDefault();
  const f = e.target.elements;
  const name = f.name.value.trim();
  act(() => call("PUT", `/admin/secrets/${integrationApp}/${name}`, { value: f.value.value }), `${name} is saved.`).then(() => e.target.reset());
});

try {
  await draw();
} catch (e) {
  $("page").hidden = true;
  say(e instanceof Refused ? `${e.message} — the admin page is the admins' alone.` : e.message, true);
}
