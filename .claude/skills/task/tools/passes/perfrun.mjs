// Cold start + screen loads + scroll smoothness on the release build with identifier
// com.jobertsvault.perf (its own data folder). Usage: node perfrun.mjs [runs]
// Build it first:  npm run tauri build -- --no-bundle --config '{"identifier":"com.jobertsvault.perf"}'
// A plain `npm run tauri build` writes the same exe with the real identifier, so this refuses to run it.
import { spawn } from "node:child_process";
import { readFileSync } from "node:fs";

const EXE = "C:/Projects/JobertsVault/src-tauri/target/release/jobertsvault.exe";
if (!readFileSync(EXE).includes("com.jobertsvault.perf")) {
  console.error(`Refusing: ${EXE} isn't the perf build, and would open the real database. See this file's header for the build.`);
  process.exit(1);
}
const PORT = 9333;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function connect(deadline) {
  for (;;) {
    if (performance.now() > deadline) throw new Error("no CDP page");
    try {
      const targets = await (await fetch(`http://127.0.0.1:${PORT}/json`)).json();
      const page = targets.find((t) => t.type === "page");
      if (page) {
        const ws = new WebSocket(page.webSocketDebuggerUrl);
        await new Promise((ok, fail) => ((ws.onopen = ok), (ws.onerror = fail)));
        let id = 0;
        const pending = new Map();
        ws.onmessage = (e) => {
          const m = JSON.parse(e.data);
          if (m.id && pending.has(m.id)) (pending.get(m.id)(m), pending.delete(m.id));
        };
        const send = (method, params = {}) =>
          new Promise((ok) => {
            pending.set(++id, ok);
            ws.send(JSON.stringify({ id, method, params }));
          });
        const evaluate = async (expression) =>
          (await send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true })).result?.result?.value;
        return { ws, send, evaluate };
      }
    } catch {}
    await sleep(10);
  }
}

async function until(app, expr, timeout = 15000) {
  const end = performance.now() + timeout;
  while (performance.now() < end) {
    if (await app.evaluate(expr)) return performance.now();
    await sleep(5);
  }
  throw new Error("timeout: " + expr);
}

const DASH_READY = `!!document.querySelector('[aria-busy="false"]') && document.body.innerText.includes('Net balance')`;
const LIST_READY = `(() => { const l = document.querySelector('[aria-busy="false"]'); return !!l && location.pathname.startsWith('/transactions') && document.querySelectorAll('main li, main [role="listitem"], main button').length > 20 })()`;

// Frame times while the list scrolls by wheel events for ~2 s.
const SCROLL = `new Promise((done) => {
  const box = [...document.querySelectorAll('main *')].find((e) => e.scrollHeight > e.clientHeight + 50 && getComputedStyle(e).overflowY !== 'visible');
  if (!box) return done({ error: 'no scroller' });
  const frames = []; let last = performance.now(); const start = last;
  const tick = (t) => { frames.push(t - last); last = t;
    box.scrollTop += 40;
    if (box.scrollTop + box.clientHeight >= box.scrollHeight - 2) box.scrollTop = 0;
    if (t - start < 2000) requestAnimationFrame(tick); else {
      frames.shift(); frames.sort((a, b) => a - b);
      done({ frames: frames.length, p50: frames[frames.length >> 1].toFixed(1), p95: frames[Math.floor(frames.length * 0.95)].toFixed(1), max: frames.at(-1).toFixed(1), over20: frames.filter((f) => f > 20).length });
    } };
  requestAnimationFrame(tick);
})`;

async function run(i) {
  const t0 = performance.now();
  const child = spawn(EXE, [], {
    env: { ...process.env, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${PORT}` },
    stdio: "ignore",
  });
  try {
    const app = await connect(t0 + 15000);
    const tConnect = performance.now();
    const tDash = await until(app, DASH_READY);
    const dashScroll = await app.evaluate(SCROLL);
    // Click the sidebar's Transactions link and time the list.
    const t1 = performance.now();
    await app.evaluate(`[...document.querySelectorAll('nav a')].find((a) => a.textContent.includes('Transactions')).click()`);
    const tList = await until(app, LIST_READY);
    await sleep(300);
    const listScroll = await app.evaluate(SCROLL);
    const size = await app.evaluate(`innerWidth + 'x' + innerHeight + ' dpr ' + devicePixelRatio`);
    console.log(
      `run ${i}: CDP up ${(tConnect - t0).toFixed(0)} ms | dashboard shown ${(tDash - t0).toFixed(0)} ms after launch | ` +
        `transactions list ${(tList - t1).toFixed(0)} ms after click | ${size}`,
    );
    console.log("  dashboard scroll", JSON.stringify(dashScroll));
    console.log("  list scroll     ", JSON.stringify(listScroll));
    app.ws.close();
  } finally {
    child.kill();
    await sleep(1500);
  }
}

const runs = Number(process.argv[2] ?? 3);
for (let i = 1; i <= runs; i++) await run(i);
