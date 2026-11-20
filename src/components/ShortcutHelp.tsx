import { useState } from "react";
import { useShortcuts } from "@/hooks/useShortcuts";
import { CHECKIN_NOTE, SHORTCUTS, showKey } from "@/lib/shortcuts";
import { DialogBody, Dialog, DialogContent } from "./ui/dialog";

/**
 * `?` shows the keyboard shortcuts (UI_FLOWS §10). It lives in `Root`, so it works on the check-in
 * too, over it like quick-add: it never navigates, and Esc closes only this dialog (§11 item 98).
 */
export function ShortcutHelp() {
  const [open, setOpen] = useState(false);
  useShortcuts({ "?": () => setOpen(true) });
  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogContent title="Keyboard shortcuts" size="list">
        <DialogBody className="pb-5">
          <table className="w-full text-left">
            <thead className="text-small text-muted-foreground">
              <tr>
                <th className="py-1 pr-3 font-normal">Shortcut</th>
                <th className="py-1 pr-3 font-normal">Action</th>
                <th className="py-1 font-normal">Where</th>
              </tr>
            </thead>
            <tbody>
              {SHORTCUTS.map(({ keys, action, where }) => (
                <tr
                  key={action}
                  className="border-t border-hairline align-baseline"
                >
                  <td className="py-1.5 pr-3 whitespace-nowrap">
                    {keys.map((key, i) => (
                      <span key={key}>
                        {i > 0 && " / "}
                        <kbd className="rounded-[6px] bg-wash px-1.5 py-0.5 font-sans text-small font-semibold">
                          {showKey(key)}
                        </kbd>
                      </span>
                    ))}
                  </td>
                  <td className="py-1.5 pr-3">{action}</td>
                  <td className="py-1.5 text-muted-foreground">{where}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <p className="text-small text-muted-foreground">{CHECKIN_NOTE}</p>
        </DialogBody>
      </DialogContent>
    </Dialog>
  );
}
