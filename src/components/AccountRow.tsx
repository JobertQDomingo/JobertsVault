import {
  CreditCard,
  Landmark,
  Smartphone,
  Wallet,
  type LucideIcon,
} from "lucide-react";
import type { ReactNode } from "react";
import type { Account } from "@/bindings/Account";
import type { AccountKind } from "@/bindings/AccountKind";
import { formatShortDate } from "@/lib/dates";
import { named, offered } from "@/lib/records";
import { cn } from "@/lib/utils";
import { MoneyText } from "./MoneyText";
import type { ComboboxOption } from "./ui/combobox";

/** DESIGN_SYSTEM §6 fixed icons. */
export const ACCOUNT_ICONS: Record<AccountKind, LucideIcon> = {
  cash: Wallet,
  bank: Landmark,
  ewallet: Smartphone,
  credit_card: CreditCard,
};

/**
 * An account filter's choices (Transactions, Reports): All accounts, then each account with its
 * icon. Archived ones are left out unless `selected` (DESIGN_SYSTEM §9.1).
 */
export const accountOptions = (
  accounts: Account[] | undefined,
  selected: number | null,
): ComboboxOption[] => [
  { value: "", label: "All accounts" },
  ...offered(accounts, selected).map((a) => {
    const Icon = ACCOUNT_ICONS[a.kind];
    return {
      value: String(a.id),
      label: named(a),
      icon: <Icon aria-hidden className="size-4 text-muted-foreground" />,
    };
  }),
];

export const ACCOUNT_KINDS: Record<AccountKind, string> = {
  cash: "Cash",
  bank: "Bank",
  ewallet: "E-wallet",
  credit_card: "Credit card",
};

/**
 * The account's icon in a `--wash` square, its name and its balance (DESIGN_SYSTEM §9.2). A credit
 * card shows what's owed, with "Available ₱X · Due Nov 5" under its name, unless `subtitle`
 * replaces that line.
 */
export function AccountRow({
  account,
  subtitle,
  className,
}: {
  account: Account;
  subtitle?: ReactNode;
  className?: string;
}) {
  const Icon = ACCOUNT_ICONS[account.kind];
  const card = account.card;
  return (
    <div
      className={cn(
        "grid grid-cols-[32px_minmax(0,1fr)_auto] items-center gap-2.5 px-[18px] py-2.5",
        className,
      )}
    >
      <span className="grid size-8 place-items-center rounded-chip bg-wash text-muted-foreground">
        <Icon aria-hidden className="size-4" />
      </span>
      <div className="min-w-0">
        <p className="truncate">{account.name}</p>
        {subtitle ? (
          <p className="text-small text-muted-foreground">{subtitle}</p>
        ) : (
          card && (
            <p className="text-small text-muted-foreground">
              Available{" "}
              <MoneyText
                amount={card.availableCredit}
                kind="balance"
                // Past the limit it's negative, and keeps its red.
                className={cn(
                  "font-normal",
                  card.availableCredit >= 0 && "text-muted-foreground",
                )}
              />{" "}
              · Due {formatShortDate(card.dueDate)}
            </p>
          )
        )}
      </div>
      {/* An overpaid card has a credit, which shows as a plain balance. */}
      {card && account.balance <= 0 ? (
        <MoneyText amount={card.amountOwed} kind="owed" />
      ) : (
        <MoneyText amount={account.balance} kind="balance" />
      )}
    </div>
  );
}
