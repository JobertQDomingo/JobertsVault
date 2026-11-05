import {
  createContext,
  useCallback,
  useContext,
  useState,
  type ReactNode,
} from "react";
import { Dialog, DialogContent } from "./ui/dialog";

const QuickAddContext = createContext<() => void>(() => {});

/** Opens quick-add (UI_FLOWS §4) from any screen: + Add, Ctrl+N, a row, Pay bill. */
export const useQuickAdd = () => useContext(QuickAddContext);

/** Holds the one quick-add dialog for the whole app. ROADMAP 1.9 fills in its form. */
export function QuickAddProvider({ children }: { children: ReactNode }) {
  const [open, setOpen] = useState(false);
  // Stable, so opening the dialog doesn't re-render every screen that can open it.
  const show = useCallback(() => setOpen(true), []);
  return (
    <QuickAddContext.Provider value={show}>
      {children}
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent title="Add transaction" />
      </Dialog>
    </QuickAddContext.Provider>
  );
}
