import { Copy, Pencil } from "lucide-react";
import type { Account } from "@/bindings/Account";
import type { Category } from "@/bindings/Category";
import type { Transaction } from "@/bindings/Transaction";
import { cn } from "@/lib/utils";
import { CategoryChip, TransferChip } from "./CategoryChip";
import { MoneyText } from "./MoneyText";
import { Button } from "./ui/button";

/**
 * One transaction (DESIGN_SYSTEM §9.2): chip │ category with the payee and note │ account, or
 * "Bank → Cash" │ amount. Hover shows ✎ and ⧉; clicking the row also opens it. `compact` is the
 * dashboard's version: chip │ category · payee │ amount, and the whole row is the button.
 */
export function TransactionRow({
  transaction: t,
  category,
  account,
  toAccount,
  compact = false,
  onOpen,
  onDuplicate,
}: {
  transaction: Transaction;
  category?: Category;
  account?: Account;
  toAccount?: Account;
  compact?: boolean;
  onOpen?: () => void;
  onDuplicate?: () => void;
}) {
  const transfer = t.kind === "transfer";
  // Categories in use can't be deleted, so a missing one is only a list that hasn't reloaded yet.
  const title = transfer ? "Transfer" : (category?.name ?? "");
  const accounts = transfer
    ? `${account?.name ?? ""} → ${toAccount?.name ?? ""}`
    : (account?.name ?? "");
  const chip = transfer ? (
    <TransferChip />
  ) : (
    <CategoryChip
      color={category?.color ?? "cat-11"}
      icon={category?.icon ?? "ellipsis"}
    />
  );
  const amount = (
    <MoneyText amount={t.amount} kind={t.kind} className="justify-self-end" />
  );
  const rowClasses =
    "grid min-h-[52px] w-full items-center gap-3 border-t border-hairline px-[18px] py-1.5 text-left transition-colors duration-150 hover:bg-glass-hover";

  if (compact) {
    const label = transfer
      ? accounts
      : [title, t.payee].filter(Boolean).join(" · ");
    const Root = onOpen ? "button" : "div";
    return (
      <Root
        {...(onOpen ? { type: "button" as const, onClick: onOpen } : {})}
        className={cn(rowClasses, "grid-cols-[32px_minmax(0,1fr)_auto]")}
      >
        {chip}
        <p className="truncate">{label}</p>
        {amount}
      </Root>
    );
  }

  const detail = [t.payee, t.note].filter(Boolean).join(" · ");
  return (
    <div
      onClick={onOpen}
      className={cn(
        rowClasses,
        "group grid-cols-[32px_minmax(0,1fr)_170px_120px_72px]",
        onOpen && "cursor-pointer",
      )}
    >
      {chip}
      <div className="min-w-0">
        <p className="truncate font-semibold">{title}</p>
        {detail && (
          <p className="truncate text-small text-muted-foreground">{detail}</p>
        )}
      </div>
      <p className="truncate text-muted-foreground">{accounts}</p>
      {amount}
      {/* Shown on hover, and whenever one has keyboard focus. */}
      <div className="flex justify-end gap-0.5 opacity-0 transition-opacity duration-150 group-focus-within:opacity-100 group-hover:opacity-100">
        <Button
          variant="ghost"
          size="icon"
          aria-label="Edit"
          title="Edit"
          onClick={(e) => {
            e.stopPropagation();
            onOpen?.();
          }}
        >
          <Pencil />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          aria-label="Duplicate"
          title="Duplicate"
          onClick={(e) => {
            e.stopPropagation();
            onDuplicate?.();
          }}
        >
          <Copy />
        </Button>
      </div>
    </div>
  );
}
