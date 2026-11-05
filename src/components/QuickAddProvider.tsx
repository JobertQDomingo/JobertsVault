import { createContext, useContext, useState, type ReactNode } from "react";
import { Dialog, DialogContent } from "./ui/dialog";

const QuickAddContext = createContext<() => void>(() => {});

/** Opens quick-add (UI_FLOWS §4) from any screen: + Add, Ctrl+N, a row, Pay bill. */
export const useQuickAdd = () => useContext(QuickAddContext);

/** Holds the one quick-add dialog for the whole app. ROADMAP 1.9 fills in its form. */
export function QuickAddProvider({ children }: { children: ReactNode }) {
  const [open, setOpen] = useState(false);
  return (
    <QuickAddContext.Provider value={() => setOpen(true)}>
      {children}
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent title="Add transaction" />
      </Dialog>
    </QuickAddContext.Provider>
  );
}
