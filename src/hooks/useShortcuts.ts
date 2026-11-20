import { useEffect, useEffectEvent } from "react";
import type { ShortcutKey } from "@/lib/shortcuts";

/**
 * App-level keyboard shortcuts (UI_FLOWS §10), named like `"Ctrl+N"`, `"Ctrl+,"`, `"?"` or
 * `"Alt+ArrowLeft"`, and only from the list in `lib/shortcuts.ts`. A match always stops WebView2's
 * own action (Ctrl+N opens a window, Ctrl+F finds), and does nothing while a dialog, popover or
 * menu is open, so it never acts behind one. A key without Ctrl or Alt is left alone while typing
 * in a field. Keys that only work inside a dialog (quick-add's Alt+E/I/T) belong to that dialog's
 * own handler.
 */
export function useShortcuts(keys: Partial<Record<ShortcutKey, () => void>>) {
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
      // `?` is Shift+/ on most layouts but not all, so the character alone names it.
      e.shiftKey && key !== "?" && "Shift",
      key,
    ]
      .filter(Boolean)
      .join("+");
    const run = keys[name as ShortcutKey];
    if (!run) return;
    const typing = (e.target as Element | null)?.closest?.(
      "input,textarea,select,[contenteditable]",
    );
    if (typing && !e.ctrlKey && !e.altKey) return;
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
