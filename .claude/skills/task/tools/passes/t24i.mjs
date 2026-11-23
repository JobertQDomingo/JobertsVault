// T24 + T25: every visible text run on every screen, in both themes and with transparency off.
// Saves each text run's box, color, size and weight, then a screenshot with all text made
// transparent, for t24text.py to measure the contrast at each run's worst spot. Also counts the
// blurred surfaces (backdrop-filter) on screen. The dashboard's answer is rewritten so every
// banner and two focus rows show (nothing is written to the database).
//   T24_DIR=<folder> node .claude/skills/task/tools/cdp.mjs .claude/skills/task/tools/passes/t24i.mjs
import { writeFileSync, mkdirSync } from "node:fs";
import { join } from "node:path";
import { onboarded } from "./t24lib.mjs";

const DIR = process.env.T24_DIR;
const SCREENS = (process.env.T24_SCREENS ?? "/,/?dialog=quick-add,/?dialog=help,/transactions,/accounts,/accounts/4,/reports,/recurring,/budgets,/goals,/debt-free,/forecast,/level,/settings,/settings?tab=Appearance").split(",");

const RUNS = `(() => {
  const out = [];
  // Behind an open modal dialog, the app is dimmed and inert: only the dialog (and toasts) count.
  const dlg = document.querySelector("[role=dialog],[role=alertdialog]");
  const w = document.createTreeWalker(dlg ?? document.body, NodeFilter.SHOW_TEXT);
  for (let n; (n = w.nextNode()); ) {
    if (!n.textContent.trim()) continue;
    const el = n.parentElement;
    if (!el || el.closest("[aria-hidden=true] svg, script, style, #t24probe")) continue;
    const cs = getComputedStyle(el);
    if (cs.visibility !== "visible" || cs.display === "none") continue;
    // sr-only: clipped to 1px
    let hidden = false;
    for (let p = el; p; p = p.parentElement) { const s = getComputedStyle(p); if ((s.position === "absolute" && (s.clip === "rect(0px, 0px, 0px, 0px)" || s.clipPath === "inset(50%)")) || s.opacity === "0") { hidden = true; break; } }
    if (hidden) continue;
    let op = 1;
    for (let p = el; p; p = p.parentElement) op *= +getComputedStyle(p).opacity;
    const r = document.createRange(); r.selectNodeContents(n);
    const rects = [...r.getClientRects()].filter(b => b.width > 2 && b.height > 4 && b.bottom > 0 && b.top < innerHeight && b.right > 0 && b.left < innerWidth).map(b => [b.left, b.top, b.width, b.height].map(Math.round));
    if (!rects.length) continue;
    const disabled = !!el.closest("button:disabled, [aria-disabled=true], input:disabled");
    const fill = cs.webkitTextFillColor && cs.webkitTextFillColor !== cs.color ? cs.webkitTextFillColor : cs.color;
    out.push({ text: n.textContent.trim().slice(0, 40), color: fill, size: parseFloat(cs.fontSize), weight: +cs.fontWeight, opacity: +op.toFixed(3), disabled, rects });
  }
  return out;
})()`;

const BLURRED = `(() => [...document.querySelectorAll("*")].filter(e => {
  const s = getComputedStyle(e); if (!s.backdropFilter || s.backdropFilter === "none") return false;
  const r = e.getBoundingClientRect(); return r.width > 0 && r.height > 0 && r.bottom > 0 && r.top < innerHeight && r.right > 0 && r.left < innerWidth && s.visibility === "visible";
}).map(e => (e.getAttribute("aria-label") || e.className.toString().split(" ").filter(c => /glass|backdrop|bg-overlay/.test(c)).join(".") || e.tagName) + "@" + Math.round(e.getBoundingClientRect().x) + "," + Math.round(e.getBoundingClientRect().y)))()`;

export default async (app) => {
  mkdirSync(DIR, { recursive: true });
  const restore = await onboarded(app);
  // Every dashboard banner and a second (weekly, near) focus row.
  await app.fakeCommand("get_dashboard", (real) => ({
    body: {
      ...real,
      backupDue: true,
      recurringDue: 3,
      cardsDue: [{ accountId: 4, amountOwed: 840000, days: 2 }],
      loansDue: [
        { loanId: 9001, name: "t24 Bank loan", accountId: 2, paymentId: 9001, amount: 105000, dueDate: "2026-11-15", days: -6, moreOverdue: 0, moveFirst: null },
        { loanId: 9002, name: "t24 E-wallet loan", accountId: 3, paymentId: 9002, amount: 100000, dueDate: "2026-11-24", days: 3, moreOverdue: 0, moveFirst: 80000 },
      ],
      focuses: real.focuses.length ? [real.focuses[0], { ...real.focuses[0], kind: "spending_cap", categoryId: null, start: "2026-11-16", end: "2026-11-22", target: 392000, actual: 333200, percent: 85, state: "near" }] : real.focuses,
    },
  }));
  const meta = { dpr: await app.eval("devicePixelRatio"), passes: {} };
  try {
    for (const theme of ["light", "dark"]) {
      await app.theme(theme);
      for (const off of [false, true]) {
        const key = `${theme}${off ? "-off" : ""}`;
        for (const screen of SCREENS) {
          const [path, q] = screen.split("?");
          const opt = new URLSearchParams(q ?? "");
          await app.eval(`location.href = ${JSON.stringify(opt.get("tab") || opt.get("dialog") ? path : screen)}`);
          await app.sleep(2200);
          await app.eval(`document.documentElement.classList.toggle("reduce-transparency", ${off}); true`);
          if (opt.get("tab")) await app.eval(`[...document.querySelectorAll("[role=tab]")].find(t => t.innerText.trim() === ${JSON.stringify(opt.get("tab"))})?.dispatchEvent(new MouseEvent("mousedown", { bubbles: true })); true`);
          if (opt.get("dialog") === "quick-add") await app.key("n", ["Control"]);
          if (opt.get("dialog") === "help") await app.key("?");
          await app.sleep(700);
          const name = `${key}__${screen.replace(/[^a-z0-9]+/gi, "_")}`;
          const runs = await app.eval(RUNS);
          const blurred = await app.eval(BLURRED);
          await app.eval(`(() => { const s = document.createElement("style"); s.id = "t24hide"; s.textContent = "*, *::placeholder { color: transparent !important; -webkit-text-fill-color: transparent !important; text-shadow: none !important; caret-color: transparent !important; }"; document.head.appendChild(s); return true; })()`);
          await app.sleep(150);
          const { data } = await app.send("Page.captureScreenshot", { format: "png" });
          writeFileSync(join(DIR, name + ".png"), Buffer.from(data, "base64"));
          await app.eval(`document.getElementById("t24hide")?.remove(); true`);
          meta.passes[name] = { screen, key, runs, blurred };
          app.log(`${name}: ${runs.length} text runs, ${blurred.length} blurred`);
          if (opt.get("dialog")) await app.key("Escape");
        }
      }
    }
  } finally {
    await app.fakeCommand(null);
    await app.eval(`document.documentElement.classList.remove("reduce-transparency"); true`);
    await app.theme("light");
    await restore();
    writeFileSync(join(DIR, "runs.json"), JSON.stringify(meta));
  }
};
