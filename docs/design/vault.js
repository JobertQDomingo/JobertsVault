// Shared behavior for the Jobert's Vault visual docs: icons, the preview theme switch,
// the "show later phases" switch, the "reduce transparency" switch and the Phase 5 drawings.
// Page-specific scripts run before this file.
(function () {
  function drawIcons() {
    if (!window.lucide) return;
    try {
      window.lucide.createIcons({ icons: window.lucide.icons, attrs: { "stroke-width": 1.75 } });
    } catch (e) { /* icons are decorative; the page still reads without them */ }
  }

  function read(key, fallback) {
    try { return localStorage.getItem(key) || fallback; } catch (e) { return fallback; }
  }
  function write(key, value) {
    try { localStorage.setItem(key, value); } catch (e) { /* per-viewer convenience only */ }
  }

  function applyTheme(mode) {
    document.querySelectorAll(".preview").forEach(function (el) {
      el.classList.toggle("theme-light", mode === "light");
      el.classList.toggle("theme-dark", mode === "dark");
    });
    document.querySelectorAll("[data-theme-mode]").forEach(function (b) {
      b.setAttribute("aria-pressed", String(b.getAttribute("data-theme-mode") === mode));
    });
    write("vault-preview-theme", mode);
  }

  function bindSwitch(id, bodyClass, key) {
    var box = document.getElementById(id);
    if (!box) return;
    function apply(on) {
      document.body.classList.toggle(bodyClass, on);
      box.checked = on;
      write(key, on ? "1" : "0");
    }
    box.addEventListener("change", function () { apply(box.checked); });
    apply(read(key, "0") === "1");
  }

  // Phase 5 components (DESIGN_SYSTEM §9.2), drawn from data attributes so both pages share one drawing
  var NS = "http://www.w3.org/2000/svg";
  var STATS = ["Cashflow", "Bills", "Card", "Debt", "Savings", "Habit"];
  var LEVELS = ["Tracker", "Planner", "Bill Keeper", "In the black", "Steady", "Card under control", "Two weeks ahead", "Debt-free", "One month ahead", "Vault Master"];
  function svgEl(name, attrs) {
    var e = document.createElementNS(NS, name);
    for (var k in attrs) e.setAttribute(k, attrs[k]);
    return e;
  }
  function nums(s) { return (s || "").split(",").map(function (v) { return v === "-" ? null : Number(v); }); }

  // A pointy-top hexagon with 3 px rounded corners, filling w × h
  function hexPath(w, h, r) {
    var p = [[w / 2, 0], [w, h / 4], [w, h * 3 / 4], [w / 2, h], [0, h * 3 / 4], [0, h / 4]];
    return p.map(function (v, i) {
      function toward(q) { var dx = q[0] - v[0], dy = q[1] - v[1], l = Math.hypot(dx, dy); return (v[0] + dx / l * r).toFixed(2) + " " + (v[1] + dy / l * r).toFixed(2); }
      return (i ? "L" : "M") + toward(p[(i + 5) % 6]) + "Q" + v[0].toFixed(2) + " " + v[1] + " " + toward(p[(i + 1) % 6]);
    }).join("") + "Z";
  }
  function badge(el) {
    var n = el.getAttribute("data-level"), sm = el.classList.contains("sm"), w = sm ? 22.52 : 34.64, h = sm ? 26 : 40;
    el.classList.toggle("zero", n === "0");
    el.innerHTML = '<svg viewBox="0 0 ' + w + " " + h + '" aria-hidden="true"><path d="' + hexPath(w, h, 3) + '"/></svg><b>' + n + "</b>";
  }

  function drawPhase5() {
    // StepIndicator: "Step 3 of 6 · Card"
    document.querySelectorAll(".v-steps[data-step]").forEach(function (el) {
      var at = +el.getAttribute("data-step"), of = +el.getAttribute("data-of"), html = '<span class="dots" aria-hidden="true">';
      for (var i = 1; i <= of; i++) html += (i > 1 ? '<i class="line"></i>' : "") + '<i class="dot' + (i < at ? " done" : i === at ? " now" : "") + '"></i>';
      el.innerHTML = html + '</span><span class="cap">Step ' + at + " of " + of + " · " + el.getAttribute("data-name") + "</span>";
    });
    // LevelBadge hexagons
    document.querySelectorAll(".lv-hex[data-level]").forEach(badge);
    // The sidebar's level chip: the small badge inside a ring filled by the share of the next level's conditions met
    document.querySelectorAll(".lv-chip[data-level]").forEach(function (el) {
      var n = el.getAttribute("data-level"), met = +el.getAttribute("data-met"), total = +el.getAttribute("data-total"), c = 2 * Math.PI * 16;
      var svg = svgEl("svg", { viewBox: "0 0 34 34", "aria-hidden": "true" });
      svg.appendChild(svgEl("circle", { cx: 17, cy: 17, r: 16, class: "track" }));
      if (met) svg.appendChild(svgEl("circle", { cx: 17, cy: 17, r: 16, class: "arc", "stroke-dasharray": (c * met / total).toFixed(2) + " " + c.toFixed(2), transform: "rotate(-90 17 17)" }));
      var hex = document.createElement("span");
      hex.className = "lv-hex sm"; hex.setAttribute("data-level", n); badge(hex);
      el.innerHTML = ""; el.appendChild(svg); el.appendChild(hex);
      el.setAttribute("aria-label", "Level " + n + ", " + met + " of " + total + " conditions met for level " + (+n + 1));
    });
    // LevelPath: ten nodes, reached ones with a check, the current one ringed, later ones locked
    document.querySelectorAll(".lv-path[data-level]").forEach(function (el) {
      var at = +el.getAttribute("data-level"), html = "";
      for (var i = 1; i <= 10; i++) {
        var on = i <= at;
        html += '<span class="n' + (on ? " on" : "") + (i === at ? " now" : "") + '" tabindex="0" title="Level ' + i + " · " + LEVELS[i - 1] + '"><span class="node"><i data-lucide="' + (on ? "check" : "lock") + '"></i></span><span class="no">' + i + "</span></span>";
      }
      el.innerHTML = html;
    });
    // StatHexagon: today's shape over last check-in's dashed outline; a stat that doesn't apply has no corner
    document.querySelectorAll("svg.v-hex[data-stats]").forEach(function (svg) {
      var now = nums(svg.getAttribute("data-stats")), prev = nums(svg.getAttribute("data-prev"));
      var keep = STATS.map(function (n, i) { return i; }).filter(function (i) { return now[i] != null; });
      if (keep.length < 3) return;
      var cx = 140, cy = 140, R = 96;
      function pt(k, v) { var a = -Math.PI / 2 + 2 * Math.PI * k / keep.length; return [cx + Math.cos(a) * R * v / 100, cy + Math.sin(a) * R * v / 100]; }
      function poly(vals) { return keep.map(function (i, k) { return pt(k, vals[i]).map(function (x) { return x.toFixed(1); }).join(","); }).join(" "); }
      svg.setAttribute("viewBox", "0 0 280 280");
      [25, 50, 75, 100].forEach(function (v) { svg.appendChild(svgEl("polygon", { points: poly(STATS.map(function () { return v; })), class: "grid" })); });
      keep.forEach(function (i, k) { var p = pt(k, 100); svg.appendChild(svgEl("line", { x1: cx, y1: cy, x2: p[0].toFixed(1), y2: p[1].toFixed(1), class: "grid" })); });
      if (svg.hasAttribute("data-prev")) svg.appendChild(svgEl("polygon", { points: poly(prev), class: "prev" }));
      svg.appendChild(svgEl("polygon", { points: poly(now), class: "now" }));
      keep.forEach(function (i, k) {
        var p = pt(k, 118), t = svgEl("text", { x: p[0].toFixed(1), y: (p[1] + 4).toFixed(1), "text-anchor": Math.abs(p[0] - cx) < 4 ? "middle" : p[0] > cx ? "start" : "end" });
        t.textContent = STATS[i]; svg.appendChild(t);
      });
    });
    // Trend lines (LVL-8): the last 12 snapshots, a flat line with no axes
    document.querySelectorAll("svg.v-trend[data-points]").forEach(function (svg) {
      // Its own range, at least 20 points tall, so a flat line sits in the middle rather than reading as low
      var v = nums(svg.getAttribute("data-points")), lo = Math.min.apply(null, v), hi = Math.max.apply(null, v), mid = (lo + hi) / 2, span = Math.max(hi - lo, 20);
      svg.setAttribute("viewBox", "0 0 64 20");
      svg.appendChild(svgEl("polyline", { points: v.map(function (x, k) { return (1 + k * 62 / (v.length - 1)).toFixed(1) + "," + (10 - (x - mid) / span * 16).toFixed(1); }).join(" ") }));
    });
  }

  // The level-up moment replays on request (DESIGN_SYSTEM §10)
  document.addEventListener("click", function (e) {
    var b = e.target.closest("[data-replay]");
    if (!b) return;
    var hex = b.closest(".aurora, .preview").querySelector(".lv-hex.up");
    if (hex) { hex.classList.remove("up"); void hex.offsetWidth; hex.classList.add("up"); }
  });

  document.addEventListener("click", function (e) {
    var b = e.target.closest("[data-theme-mode]");
    if (b) applyTheme(b.getAttribute("data-theme-mode"));
  });

  drawPhase5();
  drawIcons();
  applyTheme(read("vault-preview-theme", "auto"));
  bindSwitch("show-later", "show-later", "vault-show-later");
  bindSwitch("reduce-transparency", "reduce-transparency", "vault-reduce-transparency");
})();
