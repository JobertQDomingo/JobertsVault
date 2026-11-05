import { Search } from "lucide-react";
import { useState } from "react";
import { ACCOUNT_ICONS } from "@/components/AccountRow";
import { CategoryChip } from "@/components/CategoryChip";
import { GlassPanel } from "@/components/GlassPanel";
import { Button } from "@/components/ui/button";
import { Combobox, type ComboboxOption } from "@/components/ui/combobox";
import { fieldClasses } from "@/components/ui/input";
import { useAccounts } from "@/hooks/useAccounts";
import { useCategories } from "@/hooks/useCategories";
import { usePaymentMethods } from "@/hooks/usePaymentMethods";
import { useSettings } from "@/hooks/useSettings";
import { isFiltered, useTransactionFilter } from "@/hooks/useTransactionFilter";
import { named, offered } from "@/lib/records";
import { cn } from "@/lib/utils";

const TYPES: ComboboxOption[] = [
  { value: "", label: "All types" },
  { value: "expense", label: "Expense" },
  { value: "income", label: "Income" },
  { value: "transfer", label: "Transfer" },
];

/** "" (the All option) means any. */
const toId = (value: string) => (value ? Number(value) : null);

/**
 * The Transactions filters (UI_FLOWS §3, TXN-4) on their own glass toolbar: search in the payee
 * and note, type, account, category and payment method (only while SET-6 shows the field), and
 * Clear. Archived accounts, categories and methods aren't offered (DESIGN_SYSTEM §9.1), unless the
 * URL already filters by one.
 */
export function FilterToolbar() {
  const [filter, change, clear] = useTransactionFilter();
  const { data: settings } = useSettings();
  const { data: accounts } = useAccounts();
  const { data: categories } = useCategories();
  const { data: methods } = usePaymentMethods();
  // While the box has focus it shows what's typed, so a URL update that lands a moment later
  // never moves the cursor; otherwise it shows the URL (after Clear or Back).
  const [typing, setTyping] = useState<string | null>(null);

  const combo = "w-40 shrink-0";
  return (
    <GlassPanel className="flex items-center gap-2 rounded-toolbar p-1.5">
      <div className="relative min-w-0 flex-1">
        <Search
          aria-hidden
          className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground"
        />
        <input
          type="search"
          aria-label="Search payee or note"
          placeholder="Search payee or note…"
          title="Search (Ctrl+F)"
          aria-keyshortcuts="Control+F"
          className={cn(fieldClasses, "pl-9")}
          value={typing ?? filter.search ?? ""}
          onFocus={() => setTyping(filter.search ?? "")}
          onBlur={() => setTyping(null)}
          onChange={(e) => {
            setTyping(e.target.value);
            change({ search: e.target.value }, { replace: true });
          }}
        />
      </div>
      <Combobox
        aria-label="Type"
        placeholder="Type"
        className={combo}
        options={TYPES}
        value={filter.kind}
        onChange={(kind) =>
          change({ kind: (kind || null) as typeof filter.kind })
        }
      />
      <Combobox
        aria-label="Account"
        placeholder="Account"
        className={combo}
        options={[
          { value: "", label: "All accounts" },
          ...offered(accounts?.accounts, filter.accountId).map((a) => {
            const Icon = ACCOUNT_ICONS[a.kind];
            return {
              value: String(a.id),
              label: named(a),
              icon: (
                <Icon aria-hidden className="size-4 text-muted-foreground" />
              ),
            };
          }),
        ]}
        value={filter.accountId === null ? null : String(filter.accountId)}
        onChange={(id) => change({ accountId: toId(id) })}
      />
      <Combobox
        aria-label="Category"
        placeholder="Category"
        className={combo}
        options={[
          { value: "", label: "All categories" },
          ...offered(categories?.categories, filter.categoryId).map((c) => ({
            value: String(c.id),
            label: named(c),
            icon: <CategoryChip color={c.color} icon={c.icon} size="sm" />,
          })),
        ]}
        value={filter.categoryId === null ? null : String(filter.categoryId)}
        onChange={(id) => change({ categoryId: toId(id) })}
      />
      {settings?.showPaymentMethod && (
        <Combobox
          aria-label="Method"
          placeholder="Method"
          className={combo}
          options={[
            { value: "", label: "All methods" },
            ...offered(methods, filter.paymentMethodId).map((m) => ({
              value: String(m.id),
              label: named(m),
            })),
          ]}
          value={
            filter.paymentMethodId === null
              ? null
              : String(filter.paymentMethodId)
          }
          onChange={(id) => change({ paymentMethodId: toId(id) })}
        />
      )}
      <Button variant="ghost" onClick={clear} disabled={!isFiltered(filter)}>
        Clear
      </Button>
    </GlassPanel>
  );
}
