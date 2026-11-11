import {
  createContext,
  useContext,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import type { DueOccurrence } from "@/bindings/DueOccurrence";
import type { RecurringRule } from "@/bindings/RecurringRule";
import { QuickAddDialog, type QuickAddRequest } from "./QuickAddDialog";

type QuickAdd = {
  add: () => void;
  edit: (id: number) => void;
  /** The form starts as a copy of that transaction, dated today (TXN-2). */
  duplicate: (id: number) => void;
  /** A transfer to the card for what's owed (CC-9). */
  payBill: (cardId: number) => void;
  /** Edit & post: the form starts as the due date's transaction, and saving posts it (F10). */
  post: (due: DueOccurrence) => void;
  /** A new recurring rule, or one to edit: the form plus a schedule (UI_FLOWS §8.2). */
  rule: (rule?: RecurringRule) => void;
};

const QuickAddContext = createContext<QuickAdd>({
  add: () => {},
  edit: () => {},
  duplicate: () => {},
  payBill: () => {},
  post: () => {},
  rule: () => {},
});

/**
 * Opens quick-add (UI_FLOWS §4) from any screen: + Add and Ctrl+N, a row to edit or copy it, a
 * card's Pay bill, a due recurring date's Edit & post, or a recurring rule.
 */
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
      payBill: (cardId) => setOpen({ payBill: cardId }),
      post: (due) => setOpen({ post: due }),
      rule: (rule) => setOpen({ rule: rule ?? true }),
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
