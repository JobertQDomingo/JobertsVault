import { useQueryClient } from "@tanstack/react-query";
import { Ellipsis } from "lucide-react";
import { useState } from "react";
import { useNavigate, useParams } from "react-router";
import type { Account } from "@/bindings/Account";
import { ACCOUNT_ICONS } from "@/components/AccountRow";
import { ErrorState } from "@/components/ErrorState";
import { GlassPanel } from "@/components/GlassPanel";
import { Loading, Skeleton } from "@/components/Loading";
import { MoneyText } from "@/components/MoneyText";
import { PeriodSwitcher } from "@/components/PeriodSwitcher";
import { ProgressBar } from "@/components/ProgressBar";
import { Button } from "@/components/ui/button";
import { ConfirmDialog } from "@/components/ui/dialog";
import { Menu, MenuContent, MenuItem, MenuTrigger } from "@/components/ui/menu";
import { notify } from "@/components/ui/toast";
import { TransactionList } from "@/features/transactions/TransactionList";
import {
  useAccount,
  useArchiveAccount,
  useDeleteAccount,
  useUnarchiveAccount,
} from "@/hooks/useAccounts";
import { usePeriodQuery } from "@/hooks/usePeriodQuery";
import { useSettings } from "@/hooks/useSettings";
import { formatShortDate } from "@/lib/dates";
import { formatMoney } from "@/lib/money";
import { queryKeys } from "@/lib/queryKeys";
import { AccountDialog } from "./AccountDialog";

/**
 * `/accounts/:id` (UI_FLOWS §5.3): the balance, or a card's figures (CC-6), the ⋯ menu, and the
 * account's own transactions (incoming transfers too) under the period switcher.
 */
export function AccountDetailsScreen() {
  const id = Number(useParams().id);
  const account = useAccount(id);
  const [period] = usePeriodQuery();

  if (account.isError)
    return (
      <GlassPanel>
        <ErrorState
          onRetry={() => account.refetch()}
          pending={account.isFetching}
        />
      </GlassPanel>
    );
  return (
    <>
      {account.data ? (
        <Summary account={account.data} />
      ) : (
        <Loading>
          <GlassPanel className="grid gap-3 p-5">
            <Skeleton className="h-8 w-48" />
            <Skeleton className="h-10 w-64" />
          </GlassPanel>
        </Loading>
      )}
      <PeriodSwitcher />
      <TransactionList
        filter={{
          period,
          kind: null,
          accountId: id,
          categoryId: null,
          paymentMethodId: null,
          search: null,
        }}
        filtered={false}
        onClearFilters={() => {}}
      />
    </>
  );
}

function Summary({ account }: { account: Account }) {
  const settings = useSettings().data;
  const money = (amount: number) =>
    settings ? formatMoney(amount, settings) : "";
  const Icon = ACCOUNT_ICONS[account.kind];
  const card = account.card;
  // An overpaid card has a credit, which shows as a plain balance (as in AccountRow). The owed
  // amount is positive here, and "owed" is written beside it in the mockup's smaller muted text.
  const owed = card && account.balance <= 0;
  return (
    <GlassPanel>
      <div className="flex items-center justify-between gap-3 px-5 pt-4 pb-1">
        <h2 className="flex min-w-0 items-center gap-2.5 text-h2">
          <span className="grid size-8 shrink-0 place-items-center rounded-chip bg-wash text-muted-foreground">
            <Icon aria-hidden className="size-4" />
          </span>
          <span className="truncate">{account.name}</span>
        </h2>
        <div className="flex items-center gap-2">
          {account.archived && (
            <span className="text-small text-muted-foreground">Archived</span>
          )}
          <Actions account={account} money={money} />
        </div>
      </div>
      <div className="grid grid-cols-2 gap-5 px-5 pt-3 pb-5">
        <div className="grid content-start gap-2">
          <p className="flex items-baseline gap-2">
            {owed ? (
              <>
                <MoneyText
                  amount={card.amountOwed}
                  kind="balance"
                  size="display"
                />
                <span className="font-semibold text-muted-foreground">
                  owed
                </span>
              </>
            ) : (
              <MoneyText
                amount={account.balance}
                kind="balance"
                size="display"
              />
            )}
          </p>
          {card && account.creditLimit !== null && (
            <ProgressBar
              percent={card.utilizationPercent}
              label={`${card.utilizationPercent}% of ${money(account.creditLimit)} limit`}
              aria-label="Credit used"
            />
          )}
        </div>
        {card && (
          <table className="w-full text-small">
            <tbody className="[&_td]:border-t [&_td]:border-hairline [&_td]:py-1.5 [&_tr:first-child_td]:border-t-0">
              {[
                [
                  "Available credit",
                  <MoneyText
                    key="a"
                    amount={card.availableCredit}
                    kind="balance"
                  />,
                ],
                [
                  `This cycle (${formatShortDate(card.cycleStart)} – ${formatShortDate(card.cycleEnd)})`,
                  <>
                    <MoneyText amount={card.cycleSpending} kind="balance" />{" "}
                    spent
                  </>,
                ],
                [
                  "Statement closes",
                  <b key="s">{formatShortDate(card.cycleEnd)}</b>,
                ],
                ["Payment due", <b key="d">{formatShortDate(card.dueDate)}</b>],
              ].map(([label, value]) => (
                <tr key={String(label)}>
                  <td className="text-muted-foreground">{label}</td>
                  <td className="text-right">{value}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </GlassPanel>
  );
}

/** The ⋯ menu: Edit, Archive or Unarchive, and Delete while nothing uses the account (F6). */
function Actions({
  account,
  money,
}: {
  account: Account;
  money: (amount: number) => string;
}) {
  const client = useQueryClient();
  const navigate = useNavigate();
  const archive = useArchiveAccount();
  const unarchive = useUnarchiveAccount();
  // Gone before the refresh, so the deleted account isn't fetched again (ROADMAP 1.9). The toast
  // is here too: leaving unmounts this menu, and an unmounted mutate() skips its own callbacks.
  const remove = useDeleteAccount((id) => {
    client.removeQueries({ queryKey: queryKeys.account(id) });
    navigate("/accounts");
    notify.success("Deleted");
  });
  const [open, setOpen] = useState<"edit" | "archive" | "delete" | null>(null);
  const failed = (error: Error) => notify.error(error.message);
  const hasMoney = account.balance !== 0;
  const left =
    account.card && account.balance < 0
      ? `${money(account.card.amountOwed)} owed`
      : money(account.balance);

  return (
    <>
      <Menu>
        <MenuTrigger asChild>
          <Button variant="ghost" size="icon" aria-label="More actions">
            <Ellipsis />
          </Button>
        </MenuTrigger>
        <MenuContent>
          <MenuItem onSelect={() => setOpen("edit")}>Edit</MenuItem>
          {account.archived ? (
            <MenuItem
              onSelect={() =>
                unarchive.mutate(account.id, {
                  onSuccess: () => notify.success("Unarchived"),
                  onError: failed,
                })
              }
            >
              Unarchive
            </MenuItem>
          ) : (
            <MenuItem onSelect={() => setOpen("archive")}>Archive</MenuItem>
          )}
          {!account.inUse && (
            <MenuItem destructive onSelect={() => setOpen("delete")}>
              Delete
            </MenuItem>
          )}
        </MenuContent>
      </Menu>
      {open === "edit" && (
        <AccountDialog account={account} onClose={() => setOpen(null)} />
      )}
      <ConfirmDialog
        open={open === "archive"}
        onOpenChange={(o) => !o && setOpen(null)}
        title={`Archive ${account.name}?`}
        message={
          hasMoney
            ? `${account.name} still has ${left}. Archived accounts still count toward your net balance. Archive anyway?`
            : "It will be hidden from lists but its history stays."
        }
        confirmLabel={hasMoney ? "Archive anyway" : "Archive"}
        onConfirm={() =>
          archive.mutate(
            { id: account.id, acknowledgeBalance: hasMoney },
            { onSuccess: () => notify.success("Archived"), onError: failed },
          )
        }
      />
      <ConfirmDialog
        open={open === "delete"}
        onOpenChange={(o) => !o && setOpen(null)}
        title={`Delete ${account.name}?`}
        message="This can't be undone."
        confirmLabel="Delete"
        destructive
        onConfirm={() => remove.mutate(account.id, { onError: failed })}
      />
    </>
  );
}
