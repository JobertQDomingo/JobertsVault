import {
  createContext,
  useContext,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { Dialog, DialogContent } from "./ui/dialog";

type QuickAdd = { add: () => void; edit: (id: number) => void };

const QuickAddContext = createContext<QuickAdd>({
  add: () => {},
  edit: () => {},
});

/** Opens quick-add (UI_FLOWS §4) from any screen: + Add and Ctrl+N, or a row to edit it. */
export const useQuickAdd = () => useContext(QuickAddContext);

/** Holds the one quick-add dialog for the whole app. ROADMAP 1.9 fills in its form. */
export function QuickAddProvider({ children }: { children: ReactNode }) {
  // null: closed; { editing: null }: adding; { editing: id }: editing that transaction.
  const [open, setOpen] = useState<{ editing: number | null } | null>(null);
  // Stable, so opening the dialog doesn't re-render every screen that can open it.
  const actions = useMemo<QuickAdd>(
    () => ({
      add: () => setOpen({ editing: null }),
      edit: (id) => setOpen({ editing: id }),
    }),
    [],
  );
  return (
    <QuickAddContext.Provider value={actions}>
      {children}
      <Dialog open={open !== null} onOpenChange={() => setOpen(null)}>
        <DialogContent
          title={open?.editing ? "Edit transaction" : "Add transaction"}
        />
      </Dialog>
    </QuickAddContext.Provider>
  );
}
