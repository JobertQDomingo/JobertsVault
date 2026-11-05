import {
  createContext,
  useContext,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { QuickAddDialog, type QuickAddRequest } from "./QuickAddDialog";

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

/** Holds the one quick-add dialog for the whole app, mounted fresh for each opening. */
export function QuickAddProvider({ children }: { children: ReactNode }) {
  const [open, setOpen] = useState<QuickAddRequest | null>(null);
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
      {open && <QuickAddDialog request={open} onClose={() => setOpen(null)} />}
    </QuickAddContext.Provider>
  );
}
