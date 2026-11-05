import { ChevronRight, Plus } from "lucide-react";
import { useState } from "react";
import { Link } from "react-router";
import type { Account } from "@/bindings/Account";
import { ACCOUNT_KINDS, AccountRow } from "@/components/AccountRow";
import { ErrorState } from "@/components/ErrorState";
import { GlassPanel } from "@/components/GlassPanel";
import { Loading, Skeleton } from "@/components/Loading";
import { MoneyText } from "@/components/MoneyText";
import { SortableList } from "@/components/SortableList";
import { Button } from "@/components/ui/button";
import { notify } from "@/components/ui/toast";
import {
  useAccounts,
  useReorderAccounts,
  useUnarchiveAccount,
} from "@/hooks/useAccounts";
import { cn } from "@/lib/utils";
import { AccountDialog } from "./AccountDialog";

const subtitle = (a: Account) =>
  a.card
    ? `${ACCOUNT_KINDS[a.kind]} · ${a.card.utilizationPercent}% of limit`
    : ACCOUNT_KINDS[a.kind];

function AccountLink({ account }: { account: Account }) {
  return (
    <Link
      to={`/accounts/${account.id}`}
      className="block min-w-0 flex-1 rounded-chip transition-colors duration-150 hover:bg-glass-hover"
    >
      <AccountRow
        account={account}
        subtitle={subtitle(account)}
        className="pl-2"
      />
    </Link>
  );
}

/**
 * `/accounts` (UI_FLOWS §5.1): the active accounts in their order (drag ⋮⋮ or ↑/↓ to move one),
 * the net balance, and the archived ones folded away below. Archived balances still count toward
 * the net balance (ACC-5).
 */
export function AccountsScreen() {
  const accounts = useAccounts();
  const reorder = useReorderAccounts();
  const unarchive = useUnarchiveAccount();
  const [creating, setCreating] = useState(false);
  const [showArchived, setShowArchived] = useState(false);
  const data = accounts.data;

  let body;
  if (accounts.isError)
    body = (
      <ErrorState
        onRetry={() => accounts.refetch()}
        pending={accounts.isFetching}
      />
    );
  else if (!data)
    body = (
      <Loading className="grid gap-2 px-[18px] pb-4">
        {Array.from({ length: 4 }, (_, i) => (
          <Skeleton key={i} className="h-11" />
        ))}
      </Loading>
    );
  else {
    const archived = data.accounts.filter((a) => a.archived);
    body = (
      <>
        <SortableList
          items={data.accounts.filter((a) => !a.archived)}
          label={(a) => a.name}
          onReorder={(ids) =>
            reorder
              .mutateAsync(ids)
              .catch((error: Error) => notify.error(error.message))
          }
        >
          {(account, handle) => (
            <div className="flex items-center gap-1 border-t border-hairline pl-3">
              {handle}
              <AccountLink account={account} />
            </div>
          )}
        </SortableList>
        <div className="flex items-center justify-between border-t border-hairline bg-wash px-[18px] py-3 font-semibold">
          <span>Net balance</span>
          <MoneyText amount={data.netBalance} kind="balance" />
        </div>
        {archived.length > 0 && (
          <>
            <button
              type="button"
              aria-expanded={showArchived}
              onClick={() => setShowArchived(!showArchived)}
              className="flex w-full items-center gap-1.5 px-4 py-2.5 text-muted-foreground outline-none hover:text-foreground focus-visible:ring-3 focus-visible:ring-ring/40"
            >
              <ChevronRight
                aria-hidden
                className={cn(
                  "size-3.5 transition-transform duration-150",
                  showArchived && "rotate-90",
                )}
              />
              Show archived ({archived.length})
            </button>
            {showArchived &&
              archived.map((account) => (
                <div
                  key={account.id}
                  className="flex items-center gap-2 border-t border-hairline pr-[18px] pl-3"
                >
                  <AccountLink account={account} />
                  <Button
                    size="sm"
                    aria-label={`Unarchive ${account.name}`}
                    disabled={unarchive.isPending}
                    onClick={() =>
                      unarchive.mutate(account.id, {
                        onSuccess: () => notify.success("Unarchived"),
                        onError: (error) => notify.error(error.message),
                      })
                    }
                  >
                    Unarchive
                  </Button>
                </div>
              ))}
          </>
        )}
      </>
    );
  }

  return (
    <GlassPanel className="overflow-hidden">
      <div className="flex items-center justify-between gap-2 px-[18px] pt-4 pb-2.5">
        <h2 className="text-h3">Accounts</h2>
        <Button variant="primary" size="sm" onClick={() => setCreating(true)}>
          <Plus />
          New account
        </Button>
      </div>
      {body}
      {creating && <AccountDialog onClose={() => setCreating(false)} />}
    </GlassPanel>
  );
}
