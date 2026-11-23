// Build it first (never a plain `npm run tauri build`, which opens the real data):
//   npm run tauri build -- --no-bundle --config '{"identifier":"com.jobertsvault.perf"}'
// T25: on the separate perf build (com.jobertsvault.perf, 50,000 transactions and a year of
// Phase 3-5 data): each screen's load, its blurred surfaces, and frame times while its main list
// scrolls, in both themes. The dashboard also runs with every banner showing (its answer
// rewritten, nothing written).
//   CDP_PORT=9333 CDP_MATCH=tauri.localhost node .claude/skills/task/tools/cdp.mjs .claude/skills/task/tools/passes/t25.mjs

// Frame times while the biggest scroller in <main> scrolls 40 px a frame for ~2 s.
const SCROLL = `new Promise((done) => {
  const boxes = [...document.querySelectorAll('main, main *')].filter((e) => e.scrollHeight > e.clientHeight + 50 && /(auto|scroll)/.test(getComputedStyle(e).overflowY));
  const box = boxes.sort((a, b) => b.clientHeight * b.clientWidth - a.clientHeight * a.clientWidth)[0];
  if (!box) return done({ scroller: 'none: the screen fits the window' });
  const frames = []; let last = performance.now(); const start = last;
  const tick = (t) => { frames.push(t - last); last = t;
    box.scrollTop += 40;
    if (box.scrollTop + box.clientHeight >= box.scrollHeight - 2) box.scrollTop = 0;
    if (t - start < 2000) requestAnimationFrame(tick); else {
      frames.shift(); frames.sort((a, b) => a - b);
      done({ scroller: box.className.toString().slice(0, 30) + ' ' + box.clientHeight + '/' + box.scrollHeight + 'px', frames: frames.length, p50: +frames[frames.length >> 1].toFixed(1), p95: +frames[Math.floor(frames.length * 0.95)].toFixed(1), max: +frames.at(-1).toFixed(1), over20: frames.filter((f) => f > 20).length });
    } };
  requestAnimationFrame(tick);
})`;
const BLURRED = `[...document.querySelectorAll("*")].filter(e => { const s = getComputedStyle(e); const r = e.getBoundingClientRect();
  return s.backdropFilter && s.backdropFilter !== "none" && r.width > 0 && r.bottom > 0 && r.top < innerHeight && s.visibility === "visible"; }).length`;

export default async (app) => {
  const banners = (real) => ({
    body: {
      ...real,
      backupDue: true,
      recurringDue: 3,
      cardsDue: [{ accountId: 4, amountOwed: 840000, days: 2 }],
      loansDue: [
        { loanId: 9001, name: "t25 Bank loan", accountId: 2, paymentId: 9001, amount: 105000, dueDate: "2026-11-15", days: -6, moreOverdue: 0, moveFirst: null },
        { loanId: 9002, name: "t25 E-wallet loan", accountId: 3, paymentId: 9002, amount: 100000, dueDate: "2026-11-24", days: 3, moreOverdue: 0, moveFirst: 80000 },
      ],
    },
  });
  for (const theme of ["light", "dark"]) {
    await app.theme(theme);
    for (const path of ["/", "/ (every banner)", "/transactions", "/forecast", "/level"]) {
      const every = path.includes("banner");
      if (every) await app.fakeCommand("get_dashboard", banners);
      const t0 = Date.now();
      await app.eval(`location.href = "${path.split(" ")[0]}"`);
      await app.waitFor(`document.readyState === "complete" && !!document.querySelector("main") && !document.querySelector("main [aria-busy=true]") && document.querySelector("main").innerText.length > 200`, 15000);
      const ready = Date.now() - t0;
      await app.sleep(1200);
      const blurred = await app.eval(BLURRED);
      const scroll = await app.eval(SCROLL);
      app.log(`${theme} ${path}: page load to content ${ready} ms (full reload) | ${blurred} blurred | scroll ${JSON.stringify(scroll)}`);
      if (every) {
        await app.shot(`PERF-dashboard-every-banner-${theme}`);
        await app.fakeCommand(null);
      }
    }
  }
  await app.theme("light");
};
