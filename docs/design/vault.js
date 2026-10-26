// Shared behavior for the Jobert's Vault visual docs: icons, the preview theme switch,
// the "show later phases" switch and the "reduce transparency" switch.
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

  document.addEventListener("click", function (e) {
    var b = e.target.closest("[data-theme-mode]");
    if (b) applyTheme(b.getAttribute("data-theme-mode"));
  });

  drawIcons();
  applyTheme(read("vault-preview-theme", "auto"));
  bindSwitch("show-later", "show-later", "vault-show-later");
  bindSwitch("reduce-transparency", "reduce-transparency", "vault-reduce-transparency");
})();
