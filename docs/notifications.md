# Notifications and the home screen

A notification reaches a person in the site's inbox and, when they turned
notifications on for a device, as a push to it (`candle/docs/yenbot_app_host.md`
§11). Apps send them from workflows; the bell shows them.

## Sending one

A workflow of an app whose `app.yaml` says `notify: true`:

```js
await ctx.once(`${ctx.job.id}@${ctx.job.due_ms}`, () =>
  ctx.notify(record.owner, { title: "Swimming at 4", body: "Bring the towels.", open: "/a/planner/" }),
);
```

- **To**: an account's id (`record.owner` for the person whose record it
  is), `group:<name>`, or `admins`. Only those who may open the app are
  told.
- **What**: a title (up to 120 characters), a body (up to 1,000; a push
  shows the first 400) and `open`, the page a tap opens — one of the site's
  own, `/a/<app>/…`; the app's start when absent.
- **Once**: inside `ctx.once`, keyed on the job and its time, so a retry
  never sends it twice.

It answers `{accounts, devices}`: how many inboxes it was written to and how
many devices it was pushed to. Every notification is kept in the inbox
whether or not it was pushed.

## Getting them

The bell, in every app's shell and on the landing page, shows how many are
unread; pressed, it shows the inbox, turns notifications on for this device,
and stops this app's pushes (the inbox keeps them). From a page:
`app.notify.state` — `install`, `unsupported`, `off`, `blocked`, `on` — and
`await app.notify.enable()`, as the reminders template does.

- **Android, and computers**: "Turn on notifications" in the bell, once per
  device.
- **iPhone and iPad**: notifications come only once the site is on the home
  screen and opened from there. The bell — and the landing page, once —
  walks the person through it: Share, Add to Home Screen, then open it from
  there and turn notifications on.

## On the home screen

Every app can be put on the home screen as the site, opening full screen.
Nothing in the app is needed for it.

## When one does not arrive

1. Read the workflow's run: `log_grep` on `yenbot/<app>` for its line, or
   the job's runs (`GET /api/v1/admin/jobs?app=<app>`).
2. `{accounts: 0}`: the person named may not open the app, or is not
   active.
3. `{devices: 0}`: they never turned notifications on for a device — or, on
   an iPhone, not from the home screen. It is in their inbox all the same.
4. They stopped the app's pushes from its bell.
