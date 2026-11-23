// T24 contrast: lays each surface (panel, raised, wash card, field, banners…) over the whole
// window with the app's own classes, hides the app so only the aurora is behind it, and saves
// a screenshot per surface × theme × transparency, plus every token's resolved color.
// t24contrast.py then finds each token's worst contrast over every spot of the window.
//   T24_DIR=<folder> node .claude/skills/task/tools/cdp.mjs .claude/skills/task/tools/passes/t24h.mjs
import { writeFileSync, mkdirSync } from "node:fs";
import { join } from "node:path";

const DIR = process.env.T24_DIR;
const TOKENS = ["foreground", "muted-foreground", "primary", "primary-foreground", "income", "expense", "transfer", "warning", "danger", "danger-foreground", "success", "danger-banner", "wash", "hairline", "field-edge", "glass-solid"];
// Outer to inner: each entry is the class list (or a background) of one nested full-window layer.
const SURFACES = {
  panel: ["glass"],
  raised: ["glass-raised"],
  "panel>wash": ["glass", "bg:var(--wash)"],
  "panel>field": ["glass", "bg:var(--field)"],
  "raised>field": ["glass-raised", "bg:var(--field)"],
  "panel>wash>wash": ["glass", "bg:var(--wash)", "bg:var(--wash)"],
  "danger banner": ["glass glass-danger"],
  "danger banner>field": ["glass glass-danger", "bg:var(--field)"],
  "warning banner": ["glass glass-warning"],
  "warning banner>field": ["glass glass-warning", "bg:var(--field)"],
  "info banner": ["glass glass-info"],
  "panel>danger tint": ["glass", "glass-danger"],
  "panel>warning tint": ["glass", "glass-warning"],
  "net card": ["glass glass-tint"],
  "primary": ["bg:var(--primary)"],
  "danger": ["bg:var(--danger)"],
};

export default async (app) => {
  mkdirSync(DIR, { recursive: true });
  const out = { dpr: await app.eval("devicePixelRatio"), size: await app.eval("[innerWidth, innerHeight]"), tokens: {}, surfaces: Object.keys(SURFACES) };
  // Hide everything but the aurora (the fixed full-window layer painted with radial gradients).
  await app.eval(`(() => {
    const aurora = [...document.querySelectorAll("div[aria-hidden=true]")].find(d => (d.style.background || "").includes("radial-gradient"));
    if (!aurora) throw new Error("no aurora");
    for (let el = aurora; el && el !== document.body; el = el.parentElement)
      for (const sib of el.parentElement.children) if (sib !== el && !sib.contains(aurora)) sib.style.visibility = "hidden";
    const probe = document.createElement("div");
    probe.id = "t24probe";
    probe.style.cssText = "position:fixed;left:-40px;top:-40px;width:calc(100vw + 80px);height:calc(100vh + 80px);z-index:2147483647;";
    document.body.appendChild(probe);
    return true;
  })()`);
  try {
    for (const theme of ["light", "dark"]) {
      await app.theme(theme);
      await app.sleep(400);
      app.log(theme, "html class:", await app.eval("document.documentElement.className"));
      for (const off of [false, true]) {
        await app.eval(`document.documentElement.classList.toggle("reduce-transparency", ${off}); true`);
        const key = `${theme}${off ? "-off" : ""}`;
        out.tokens[key] = await app.eval(`(() => {
          const s = document.createElement("span"); document.body.appendChild(s); const r = {};
          for (const t of ${JSON.stringify(TOKENS)}) { s.style.color = "var(--" + t + ")"; r[t] = getComputedStyle(s).color; }
          s.remove(); return r;
        })()`);
        for (const [name, layers] of Object.entries(SURFACES)) {
          await app.eval(`(() => {
            const probe = document.getElementById("t24probe"); probe.innerHTML = "";
            let parent = probe;
            for (const l of ${JSON.stringify(layers)}) {
              const d = document.createElement("div");
              d.style.cssText = "position:absolute;inset:0;border-radius:0;" + (l.startsWith("bg:") ? "background:" + l.slice(3) : "");
              if (!l.startsWith("bg:")) d.className = l;
              d.style.borderRadius = "0"; d.style.border = "0"; d.style.boxShadow = "none";
              parent.appendChild(d); parent = d;
            }
            return true;
          })()`);
          await app.sleep(350);
          const { data } = await app.send("Page.captureScreenshot", { format: "png" });
          writeFileSync(join(DIR, `${key}__${name.replace(/[^a-z]+/g, "_")}.png`), Buffer.from(data, "base64"));
        }
      }
    }
  } finally {
    await app.eval(`document.getElementById("t24probe")?.remove(); document.documentElement.classList.remove("reduce-transparency"); true`);
    await app.theme("light");
    await app.eval(`location.reload(); true`);
    writeFileSync(join(DIR, "tokens.json"), JSON.stringify(out, null, 1));
  }
};
