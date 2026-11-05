import {
  createContext,
  useContext,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { Dialog, DialogContent } from "./ui/dialog";

type QuickAdd = {
  add: () => void;
  edit: (id: number) => void;
  /** The form starts as a copy of that transaction, dated today (TXN-2). */
  duplicate: (id: number) => void;
};

const QuickAddContext = createContext<QuickAdd>({
  add: () => {},
  edit: () => {},
  duplicate: () => {},
});

/** Opens quick-add (UI_FLOWS §4) from any screen: + Add and Ctrl+N, or a row to edit or copy it. */
export const useQuickAdd = () => useContext(QuickAddContext);

type Open = { edit?: number; duplicate?: number };

/** Holds the one quick-add dialog for the whole app. */
// shortcut: no form yet, so `edit` and `duplicate` only set the title; ROADMAP 1.9 task 3 fills it.
export function QuickAddProvider({ children }: { children: ReactNode }) {
  const [open, setOpen] = useState<Open | null>(null);
  // Stable, so opening the dialog doesn't re-render every screen that can open it.
  const actions = useMemo<QuickAdd>(
    () => ({
      add: () => setOpen({}),
      edit: (id) => setOpen({ edit: id }),
      duplicate: (id) => setOpen({ duplicate: id }),
    }),
    [],
  );
  return (
    <QuickAddContext.Provider value={actions}>
      {children}
      <Dialog open={open !== null} onOpenChange={() => setOpen(null)}>
        <DialogContent
          title={open?.edit != null ? "Edit transaction" : "Add transaction"}
        />
      </Dialog>
    </QuickAddContext.Provider>
  );
}
