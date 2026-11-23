// T24 helpers: a keyboard Tab walk and an accessibility-tree scan, for any screen.
//   import { tabWalk, unnamed } from "./t24lib.mjs";

// Describes document.activeElement: what a screen reader would call it, where it is, and
// whether anything marks it as focused (outline or box-shadow, its own or a parent's ring).
const ACTIVE = `(() => {
  const el = document.activeElement;
  if (!el || el === document.body) return { body: true };
  const cs = getComputedStyle(el);
  const r = el.getBoundingClientRect();
  const lab = el.getAttribute("aria-labelledby");
  const name = (el.getAttribute("aria-label") ?? (lab && lab.split(" ").map((i) => document.getElementById(i)?.innerText).join(" ")) ?? el.innerText ?? el.value ?? el.title ?? "").trim().replace(/\\s+/g, " ").slice(0, 50) || el.getAttribute("placeholder") || "";
  const ring = (s) => (s.outlineStyle !== "none" && parseFloat(s.outlineWidth) > 0) || (s.boxShadow !== "none" && s.boxShadow !== "");
  let marked = ring(cs) ? "self" : null;
  for (let p = el.parentElement, i = 0; !marked && p && i < 3; p = p.parentElement, i++) if (ring(getComputedStyle(p)) && p.matches(":focus-within")) marked = "parent";
  const dlg = el.closest("[role=dialog],[role=alertdialog]");
  return {
    tag: el.tagName.toLowerCase(), role: el.getAttribute("role") ?? "", name, checked: el.getAttribute("aria-checked"),
    x: Math.round(r.x), y: Math.round(r.y), w: Math.round(r.width), h: Math.round(r.height),
    fv: el.matches(":focus-visible"), marked, outline: cs.outlineStyle + " " + cs.outlineWidth, shadow: cs.boxShadow.slice(0, 40),
    dialog: dlg ? (dlg.getAttribute("aria-label") ?? dlg.querySelector("h2")?.innerText ?? "dialog") : null,
    inView: r.bottom > 0 && r.top < innerHeight && r.width > 0,
    key: el.tagName + "|" + (el.id || "") + "|" + r.x + "," + r.y + "|" + name,
  };
})()`;

export const active = (app) => app.eval(ACTIVE);

/** Tabs from the top of the page until focus comes back round (or `max` stops). */
export async function tabWalk(app, { max = 80, from = "body", shift = false } = {}) {
  if (from === "body") await app.eval(`document.activeElement?.blur(); window.focus(); true`);
  const stops = [];
  for (let i = 0; i < max; i++) {
    await app.key("Tab", shift ? ["Shift"] : []);
    const a = await active(app);
    if (stops.length && a.key && a.key === stops[0].key) break;
    stops.push(a);
  }
  return stops;
}

export const line = (a, i) =>
  a.body
    ? `${String(i + 1).padStart(2)}. <body>`
    : `${String(i + 1).padStart(2)}. ${a.tag}${a.role ? "[" + a.role + "]" : ""} "${a.name}" @${a.x},${a.y} ${a.w}x${a.h}` +
      `${a.marked ? "" : " NO-RING"}${a.fv ? "" : " not-fv"}${a.inView ? "" : " OFFSCREEN"}${a.dialog ? " in:" + a.dialog : ""}`;

export function printWalk(app, title, stops) {
  app.log(`--- ${title}: ${stops.length} stops`);
  stops.forEach((a, i) => app.log(line(a, i)));
}

const INTERACTIVE = new Set(["button", "link", "textbox", "searchbox", "combobox", "checkbox", "radio", "switch", "tab", "menuitem", "menuitemradio", "slider", "spinbutton", "listbox", "option", "img", "image", "radiogroup", "dialog", "alertdialog", "table", "progressbar", "meter"]);

/** Interactive or image nodes in the accessibility tree that have no name. */
export async function unnamed(app) {
  await app.send("Accessibility.enable");
  const { nodes } = await app.send("Accessibility.getFullAXTree");
  const bad = [];
  for (const n of nodes) {
    if (n.ignored) continue;
    const role = n.role?.value;
    if (!INTERACTIVE.has(role)) continue;
    const name = (n.name?.value ?? "").trim();
    if (!name) bad.push(role + (n.backendDOMNodeId ? "#" + n.backendDOMNodeId : ""));
  }
  return bad;
}

/** The accessible names of every node of the given roles, as Chromium computes them. */
export async function names(app, roles) {
  await app.send("Accessibility.enable");
  const { nodes } = await app.send("Accessibility.getFullAXTree");
  return nodes
    .filter((n) => !n.ignored && roles.includes(n.role?.value))
    .map((n) => `${n.role.value}: ${(n.name?.value ?? "").trim()}${n.value?.value ? " = " + n.value.value : ""}${n.properties?.find((p) => p.name === "checked") ? " checked=" + n.properties.find((p) => p.name === "checked").value.value : ""}`);
}

/** Describes the unnamed node's DOM element (role#backendId from `unnamed`). */
export async function describe(app, ref) {
  const id = Number(ref.split("#")[1]);
  if (!id) return ref;
  const { object } = await app.send("DOM.resolveNode", { backendNodeId: id });
  const { result } = await app.send("Runtime.callFunctionOn", {
    objectId: object.objectId,
    functionDeclaration: "function () { return this.outerHTML.slice(0, 220); }",
    returnByValue: true,
  });
  return ref + " " + result.value;
}

export const stepLine = (app) =>
  app.eval(`[...document.querySelectorAll("span")].find(s => /^Step \\d+ of/.test(s.textContent.trim()))?.textContent.trim() ?? null`);

/** From the check-in's first step to Pick a focus, by keyboard only: Enter, and Tab + Space on
 *  each unanswered "Matches" of the Balance check. */
export async function toPickAFocus(app) {
  await app.eval(`location.href = "/check-in"`);
  await app.sleep(2500);
  for (let i = 0; i < 12; i++) {
    const s = (await stepLine(app)) ?? "";
    if (/Pick a focus/.test(s)) return true;
    if (/Balance check/.test(s)) {
      await app.eval(`document.activeElement?.blur(); window.focus(); true`);
      for (let k = 0; k < 40; k++) {
        await app.key("Tab");
        const a = await active(app);
        if (a.name === "Matches" && a.checked !== "true") await app.key("Space");
        if (await app.eval(`[...document.querySelectorAll("[role=radiogroup]")].every(g => g.querySelector("[aria-checked=true]"))`)) break;
      }
    }
    await app.eval(`document.activeElement?.blur(); true`);
    await app.key("Enter");
    await app.sleep(500);
  }
  return false;
}

/** Sets onboarding done for the session; returns a function that puts it back. */
export async function onboarded(app) {
  const { decimalsLocked, ...original } = await app.invoke("get_settings");
  await app.invoke("update_settings", { settings: { ...original, onboardingCompleted: true } });
  return () => app.invoke("update_settings", { settings: original });
}
