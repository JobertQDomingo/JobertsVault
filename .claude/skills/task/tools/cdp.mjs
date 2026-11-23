// Drives the running Tauri dev app through WebView2's DevTools port (9222).
// Usage: node cdp.mjs <steps.mjs>   where steps.mjs exports `default async (app) => {}`.
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const shots = join(here, "../../../../.superpowers/shots"); // git-ignored; never inside the tracked skill folder
mkdirSync(shots, { recursive: true });

// T25: CDP_PORT=9333 CDP_MATCH=tauri.localhost drives the separate perf build instead.
const targets = await (await fetch(`http://127.0.0.1:${process.env.CDP_PORT ?? 9222}/json`)).json();
const page = targets.find((t) => t.type === "page" && t.url.includes(process.env.CDP_MATCH ?? "localhost:1420"));
if (!page) throw new Error("no app page: " + JSON.stringify(targets.map((t) => t.url)));
const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((ok, fail) => ((ws.onopen = ok), (ws.onerror = fail)));
let id = 0;
const pending = new Map();
const consoleErrors = [];
let fake = null; // { command, fn }: rewrites one command's IPC answer (see app.fakeCommand)
let stub = null; // { fn }: answers a command itself; the request never reaches Rust (app.stubCommand)
ws.onmessage = (e) => {
  const msg = JSON.parse(e.data);
  if (msg.id && pending.has(msg.id)) {
    const { ok, fail } = pending.get(msg.id);
    pending.delete(msg.id);
    msg.error ? fail(new Error(JSON.stringify(msg.error))) : ok(msg.result);
  } else if (msg.method === "Fetch.requestPaused") {
    stub && msg.params.responseStatusCode === undefined ? answerStub(msg.params) : answerPaused(msg.params);
  } else if (msg.method === "Runtime.exceptionThrown") {
    consoleErrors.push(msg.params.exceptionDetails.exception?.description ?? msg.params.exceptionDetails.text);
  } else if (msg.method === "Runtime.consoleAPICalled" && ["error", "warning"].includes(msg.params.type)) {
    consoleErrors.push(msg.params.type + ": " + msg.params.args.map((a) => a.value ?? a.description).join(" "));
  }
};
const send = (method, params = {}) =>
  new Promise((ok, fail) => {
    const n = ++id;
    pending.set(n, { ok, fail });
    ws.send(JSON.stringify({ id: n, method, params }));
  });
await send("Runtime.enable");
await send("Page.enable");

const KEYS = {
  Enter: [13, "Enter", "\r"], Escape: [27, "Escape"], Tab: [9, "Tab"], Backspace: [8, "Backspace"],
  ArrowLeft: [37, "ArrowLeft"], ArrowRight: [39, "ArrowRight"], ArrowUp: [38, "ArrowUp"], ArrowDown: [40, "ArrowDown"],
  Home: [36, "Home"], End: [35, "End"], Space: [32, "Space", " "], Delete: [46, "Delete"], ",": [188, "Comma", ","],
};
const MOD = { Alt: 1, Control: 2, Meta: 4, Shift: 8 };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function answerStub({ requestId, request }) {
  if (request.method !== "POST") return send("Fetch.continueRequest", { requestId });
  // fn may return { __error: value } to make the command fail with that error (T11).
  const out = await stub.fn(request);
  const failed = out !== null && typeof out === "object" && "__error" in out;
  const body = failed ? out.__error : out;
  await send("Fetch.fulfillRequest", {
    requestId,
    responseCode: 200,
    responseHeaders: [
      { name: "Content-Type", value: "application/json" },
      { name: "Tauri-Response", value: failed ? "error" : "ok" },
      { name: "Access-Control-Allow-Origin", value: "*" },
      { name: "Access-Control-Expose-Headers", value: "Tauri-Response" },
    ],
    body: Buffer.from(JSON.stringify(body)).toString("base64"),
  });
}

async function answerPaused({ requestId, request, responseHeaders = [], responseStatusCode }) {
  if (process.env.CDP_DEBUG) console.log("paused", request.method, request.url, responseStatusCode);
  // Only the command's answer; its CORS preflight passes through.
  if (!fake || request.method !== "POST") return send("Fetch.continueRequest", { requestId });
  const { body, base64Encoded } = await send("Fetch.getResponseBody", { requestId });
  const real = JSON.parse(base64Encoded ? Buffer.from(body, "base64").toString() : body);
  const out = await fake.fn(real); // { body } | { error } , optional { delay }
  if (out.delay) await sleep(out.delay);
  const headers = responseHeaders.filter((h) => !/^(tauri-response|content-length)$/i.test(h.name));
  headers.push({ name: "Tauri-Response", value: out.error ? "error" : "ok" });
  await send("Fetch.fulfillRequest", { requestId, responseCode: 200, responseHeaders: headers, body: Buffer.from(JSON.stringify(out.error ?? out.body)).toString("base64") });
}

const app = {
  log: (...a) => console.log(...a),
  sleep,
  async eval(expr) {
    const r = await send("Runtime.evaluate", { expression: expr, returnByValue: true, awaitPromise: true });
    if (r.exceptionDetails) throw new Error("eval: " + (r.exceptionDetails.exception?.description ?? r.exceptionDetails.text));
    return r.result.value;
  },
  async waitFor(expr, ms = 5000) {
    const end = Date.now() + ms;
    for (;;) {
      const v = await app.eval(expr).catch(() => null);
      if (v) return v;
      if (Date.now() > end) throw new Error("timed out waiting for " + expr);
      await sleep(100);
    }
  },
  // Center of the first element matching a CSS selector, or of the element whose trimmed text or
  // aria-label equals `text` (buttons, links, tabs, options, menu items, labels first).
  async locate(target) {
    const box = await app.eval(`(() => {
      const t = ${JSON.stringify(target)};
      let el;
      if (typeof t === "string") el = document.querySelector(t);
      else {
        const all = [...document.querySelectorAll(t.selector ?? 'button,a,[role=tab],[role=option],[role=menuitem],[role=radio],[role=checkbox],[role=switch],[role=combobox],label,input,[role=gridcell] button,[role=link],[role=row],li,td,div,span,p,h1,h2,h3')];
        const visible = (e) => { const r = e.getBoundingClientRect(); return r.width > 0 && r.height > 0; };
        const name = (e) => (e.getAttribute("aria-label") ?? e.innerText ?? e.value ?? "").trim().replace(/\\s+/g, " ");
        el = all.filter(visible).find((e) => t.exact === false ? name(e).includes(t.text) : name(e) === t.text);
      }
      if (!el) return null;
      el.scrollIntoView({ block: "nearest" });
      const r = el.getBoundingClientRect();
      return { x: r.x + r.width / 2, y: r.y + r.height / 2, w: r.width, h: r.height };
    })()`);
    if (!box) throw new Error("not found: " + JSON.stringify(target));
    return box;
  },
  async click(target, { button = "left", count = 1 } = {}) {
    const { x, y } = await app.locate(target);
    await send("Input.dispatchMouseEvent", { type: "mouseMoved", x, y });
    await send("Input.dispatchMouseEvent", { type: "mousePressed", x, y, button, clickCount: count });
    await send("Input.dispatchMouseEvent", { type: "mouseReleased", x, y, button, clickCount: count });
    await sleep(150);
  },
  async hover(target) {
    const { x, y } = await app.locate(target);
    await send("Input.dispatchMouseEvent", { type: "mouseMoved", x, y });
    await sleep(150);
  },
  async drag(from, to, steps = 12) {
    const a = await app.locate(from);
    const b = typeof to === "object" && "dy" in to ? { x: a.x + (to.dx ?? 0), y: a.y + to.dy } : await app.locate(to);
    await send("Input.dispatchMouseEvent", { type: "mouseMoved", x: a.x, y: a.y });
    await send("Input.dispatchMouseEvent", { type: "mousePressed", x: a.x, y: a.y, button: "left", clickCount: 1 });
    for (let i = 1; i <= steps; i++) {
      await send("Input.dispatchMouseEvent", { type: "mouseMoved", x: a.x + ((b.x - a.x) * i) / steps, y: a.y + ((b.y - a.y) * i) / steps, button: "left", buttons: 1 });
      await sleep(30);
    }
    await send("Input.dispatchMouseEvent", { type: "mouseReleased", x: b.x, y: b.y, button: "left", clickCount: 1 });
    await sleep(250);
  },
  async type(text) {
    for (const ch of text) {
      await send("Input.dispatchKeyEvent", { type: "keyDown", key: ch, text: ch, unmodifiedText: ch });
      await send("Input.dispatchKeyEvent", { type: "keyUp", key: ch });
    }
    await sleep(100);
  },
  // e.g. key("Enter"), key("n", ["Control"]), key("ArrowLeft", ["Alt"])
  async key(name, mods = []) {
    const modifiers = mods.reduce((m, k) => m | MOD[k], 0);
    let [code, codeName, text] = KEYS[name] ?? [name.toUpperCase().charCodeAt(0), /\d/.test(name) ? "Digit" + name : "Key" + name.toUpperCase(), name];
    const printable = text !== undefined && !mods.includes("Control") && !mods.includes("Alt");
    const base = { modifiers, key: name === "Space" ? " " : name, code: codeName, windowsVirtualKeyCode: code, nativeVirtualKeyCode: code };
    await send("Input.dispatchKeyEvent", { type: printable ? "keyDown" : "rawKeyDown", ...base, ...(printable ? { text } : {}) });
    await send("Input.dispatchKeyEvent", { type: "keyUp", ...base });
    await sleep(150);
  },
  async shot(name) {
    await sleep(250);
    const { data } = await send("Page.captureScreenshot", { format: "png" });
    const file = join(shots, name + ".png");
    writeFileSync(file, Buffer.from(data, "base64"));
    console.log("shot:", file);
    return file;
  },
  async theme(scheme, extra = []) {
    await send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-color-scheme", value: scheme }, ...extra] });
    await sleep(200);
  },
  async media(features) {
    await send("Emulation.setEmulatedMedia", { features });
    await sleep(200);
  },
  async go(path) {
    await app.eval(`history.pushState(null, "", ${JSON.stringify(path)}); dispatchEvent(new PopStateEvent("popstate"))`);
    await sleep(400);
  },
  async reload() {
    await send("Page.reload");
    await sleep(1500);
  },
  text: (sel = "body") => app.eval(`document.querySelector(${JSON.stringify(sel)})?.innerText ?? null`),
  invoke: (cmd, args = {}) => app.eval(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(cmd)}, ${JSON.stringify(args)})`),
  errors: () => consoleErrors.splice(0),
  // Rewrites `command`'s IPC answer: fn(realAnswer) → { body } or { error }, plus { delay }.
  // The request still reaches Rust; only what the page receives changes. fakeCommand(null) stops.
  async fakeCommand(command, fn) {
    fake = command ? { command, fn } : null;
    if (!command) return send("Fetch.disable");
    await send("Fetch.enable", { patterns: [{ urlPattern: `*/${command}*`, requestStage: "Response" }] });
  },
  // Answers IPC requests whose URL matches `pattern` (e.g. "*dialog%7Copen*") with fn(request),
  // before they reach Rust, so a native dialog never opens. stubCommand(null) stops.
  async stubCommand(pattern, fn) {
    stub = pattern ? { fn } : null;
    if (!pattern) return send("Fetch.disable");
    await send("Fetch.enable", { patterns: [{ urlPattern: pattern, requestStage: "Request" }] });
  },
  send,
};

const steps = (await import(pathToFileURL(process.argv[2]).href)).default;
try {
  await steps(app);
} finally {
  if (consoleErrors.length) console.log("console errors:", consoleErrors);
  ws.close();
}
