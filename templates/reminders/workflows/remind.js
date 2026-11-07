// A reminder's time has come: ping the person it is theirs, once, whatever
// retries follow.
export default async function remind(ctx, { record }) {
  await ctx.once(`remind:${record.id}@${record.due_ms}`, () =>
    ctx.notify(record.owner, {
      title: record.title,
      body: `It's ${ctx.time(record.due_ms)}.`,
      open: `/a/${ctx.app}/`,
    }),
  );
  ctx.log("reminded", record.owner, "of", record.title);
}
