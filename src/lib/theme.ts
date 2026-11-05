/**
 * Keeps the `.dark` class on <html> in step with Windows' light or dark setting, including
 * changes while the app is open (DESIGN_SYSTEM §3.5). Returns a function that stops it.
 */
export function followSystemTheme(root = document.documentElement) {
  const query = window.matchMedia("(prefers-color-scheme: dark)");
  const apply = () => root.classList.toggle("dark", query.matches);
  apply();
  query.addEventListener("change", apply);
  return () => query.removeEventListener("change", apply);
}
