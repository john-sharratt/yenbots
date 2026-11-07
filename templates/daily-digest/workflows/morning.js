// Each morning at 07:00 (`every: daily 07:00`), or now from the app's
// button: gather what is on in the day ahead and send it as one note — to
// the admins here; `group:household` or an account's id sends it to them.
// Sent once per job and time, whatever retries follow.
export default async function morning(ctx) {
  const now = ctx.now();
  const day = 24 * 60 * 60 * 1000;
  const { docs } = await ctx.data
    .collection("items")
    .list({ where: `due_ms >= ${now} and due_ms < ${now + day}`, order: "due_ms" });
  if (docs.length === 0) {
    ctx.log("nothing on today");
    return 0;
  }
  const body = docs.map((d) => `${ctx.time(d.due_ms)} — ${d.title}`).join("\n");
  const key = ctx.job ? `${ctx.job.id}@${ctx.job.due_ms}` : `now@${now}`;
  await ctx.once(key, () =>
    ctx.notify("admins", {
      title: docs.length === 1 ? "One thing today" : `${docs.length} things today`,
      body,
      open: `/a/${ctx.app}/`,
    }),
  );
  ctx.log("sent", docs.length, "things");
  return docs.length;
}
