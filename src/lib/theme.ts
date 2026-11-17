/**
 * Keeps the `.dark` class on <html> in step with the window's theme, including changes while the
 * app is open. Rust sets that theme from the setting (SET-5), or leaves it to Windows when it's
 * System, and WebView2 reports it as `prefers-color-scheme` (DESIGN_SYSTEM §3.5). Returns a
 * function that stops it.
 */
export function followWindowTheme(root = document.documentElement) {
  const query = window.matchMedia("(prefers-color-scheme: dark)");
  const apply = () => root.classList.toggle("dark", query.matches);
  apply();
  query.addEventListener("change", apply);
  return () => query.removeEventListener("change", apply);
}
