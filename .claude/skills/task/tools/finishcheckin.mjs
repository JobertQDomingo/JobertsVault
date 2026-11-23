// From T11 on, the dev app opens on /check-in whenever one is due. This finishes it, so a
// script can get on with its own screens. DEV database only.
//   node .claude/skills/task/tools/cdp.mjs .claude/skills/task/tools/finishcheckin.mjs
// Or from another steps file:  import finish from "./finishcheckin.mjs"; await finish(app);
// It picks the first offer, as Finish would with that card chosen, then reloads to "/".
// To make one due again, close the app and run: python -I .claude/skills/task/tools/redue.py week|month|both
export default async (app) => {
  const due = await app.invoke("get_checkin");
  if (!due) {
    app.log("no check-in due");
    return;
  }
  const written = await app.invoke("complete_checkin", { focus: due.offers[0] });
  app.log(`finished ${due.kinds.join("+")} ${due.period.start}–${due.period.end}: ${written.length} row(s)`);
  await app.eval(`location.href = "/"`);
  await app.sleep(1800);
};
