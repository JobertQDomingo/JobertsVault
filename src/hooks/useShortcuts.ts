import { useEffect, useEffectEvent } from "react";

/**
 * App-level keyboard shortcuts (UI_FLOWS §10), named like `"Ctrl+N"`, `"Ctrl+,"` or
 * `"Alt+ArrowLeft"`. A match always stops WebView2's own action (Ctrl+N opens a window, Ctrl+F
 * finds), and does nothing while a dialog, popover or menu is open, so it never acts behind one.
 * Keys that only work inside a dialog (quick-add's Alt+E/I/T) belong to that dialog's own handler.
 */
export function useShortcuts(keys: Record<string, () => void>) {
  const onKeyDown = useEffectEvent((e: KeyboardEvent) => {
    // Digits by the key's place: on AZERTY, Ctrl and the 1 key types "&".
    const key = e.code.startsWith("Digit")
      ? e.code.slice(5)
      : e.key.length === 1
        ? e.key.toUpperCase()
        : e.key;
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
    if (!document.querySelector("[role=dialog],[role=alertdialog],[role=menu]"))
      run();
  });
  useEffect(() => {
    const listener = (e: KeyboardEvent) => onKeyDown(e);
    window.addEventListener("keydown", listener);
    return () => window.removeEventListener("keydown", listener);
  }, []);
}
