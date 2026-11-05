import {
  CreditCard,
  Landmark,
  Smartphone,
  Wallet,
  type LucideIcon,
} from "lucide-react";
import type { Account } from "@/bindings/Account";
import type { AccountKind } from "@/bindings/AccountKind";
import { formatShortDate } from "@/lib/dates";
import { cn } from "@/lib/utils";
import { MoneyText } from "./MoneyText";

/** DESIGN_SYSTEM §6 fixed icons. */
export const ACCOUNT_ICONS: Record<AccountKind, LucideIcon> = {
  cash: Wallet,
  bank: Landmark,
  ewallet: Smartphone,
  credit_card: CreditCard,
};

/**
 * The account's icon in a `--wash` square, its name and its balance (DESIGN_SYSTEM §9.2). A credit
 * card shows what's owed, with "Available ₱X · Due Nov 5" under its name.
 */
export function AccountRow({
  account,
  className,
}: {
  account: Account;
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
        {card && (
          <p className="text-small text-muted-foreground">
            Available{" "}
            <MoneyText
              amount={card.availableCredit}
              kind="balance"
              className="font-normal text-muted-foreground"
            />{" "}
            · Due {formatShortDate(card.dueDate)}
          </p>
        )}
      </div>
      {card ? (
        <MoneyText amount={card.amountOwed} kind="owed" />
      ) : (
        <MoneyText amount={account.balance} kind="balance" />
      )}
    </div>
  );
}
