import { useEffect, useEffectEvent } from "react";

/**
 * Keyboard shortcuts (UI_FLOWS §10), named like `"Ctrl+N"`, `"Ctrl+,"` or `"Alt+ArrowLeft"`. A
 * match always stops WebView2's own action (Ctrl+N opens a window, Ctrl+F finds, Alt+← goes
 * back), and does nothing while a dialog or popover is open, so it never acts behind one.
 */
export function useShortcuts(keys: Record<string, () => void>) {
  const onKeyDown = useEffectEvent((e: KeyboardEvent) => {
    const key = e.key.length === 1 ? e.key.toUpperCase() : e.key;
    const name = [
      e.ctrlKey && "Ctrl",
      e.altKey && "Alt",
      e.shiftKey && "Shift",
      key,
    ]
      .filter(Boolean)
      .join("+");
    const run = keys[name];
    if (!run) return;
    e.preventDefault();
    if (!document.querySelector("[role=dialog]")) run();
  });
  useEffect(() => {
    const listener = (e: KeyboardEvent) => onKeyDown(e);
    window.addEventListener("keydown", listener);
    return () => window.removeEventListener("keydown", listener);
  }, []);
}
