import { Plus } from "lucide-react";
import { useState, type ReactNode } from "react";
import { Navigate } from "react-router";
import type { AccountKind } from "@/bindings/AccountKind";
import { ACCOUNT_ICONS, ACCOUNT_KINDS } from "@/components/AccountRow";
import { ErrorState } from "@/components/ErrorState";
import { GlassPanel } from "@/components/GlassPanel";
import { Loading, Skeleton } from "@/components/Loading";
import { Button } from "@/components/ui/button";
import { notify } from "@/components/ui/toast";
import { AccountDialog } from "@/features/accounts/AccountDialog";
import { GeneralTab } from "@/features/settings/GeneralTab";
import { useAccounts } from "@/hooks/useAccounts";
import { useSettings, useUpdateSettings } from "@/hooks/useSettings";

const ADD: AccountKind[] = ["bank", "ewallet", "credit_card"];

function Step({
  n,
  title,
  children,
}: {
  n: number;
  title?: string;
  children: ReactNode;
}) {
  return (
    <section className="grid grid-cols-[28px_minmax(0,1fr)] gap-3 border-t border-hairline py-4">
      <span
        aria-hidden
        className="grid size-7 place-items-center rounded-full bg-wash font-semibold text-muted-foreground"
      >
        {n}
      </span>
      <div className="grid gap-2">
        {title && <h2 className="text-h3">{title}</h2>}
        {children}
      </div>
    </section>
  );
}

const loading = (
  <Loading className="grid gap-2">
    {[0, 1, 2].map((i) => (
      <Skeleton key={i} className="h-9" />
    ))}
  </Loading>
);

/**
 * `/welcome` (ONB-1, UI_FLOWS F5): the plain first-launch form. The currency saves as it changes,
 * so the decimals are set before any amount is typed; accounts are added with the usual dialog.
 * Either button ends it for good (`onboarding_completed`); closing the app first shows it again.
 */
export function WelcomeScreen() {
  const settings = useSettings();
  const accounts = useAccounts();
  const update = useUpdateSettings();
  const [adding, setAdding] = useState<AccountKind | null>(null);
  // The accounts there when the form opened (Cash on a new database) are the "already added" ones.
  const [existing, setExisting] = useState<number[] | null>(null);
  if (!existing && accounts.data)
    setExisting(accounts.data.accounts.map((a) => a.id));

  // Once done (either button, or an old link) it's the dashboard, replacing this entry.
  if (settings.data?.onboardingCompleted) return <Navigate to="/" replace />;
  const finish = () =>
    update
      .mutateAsync({ onboardingCompleted: true })
      .catch((error: Error) => notify.error(error.message));
  // The finish save is laid over the loaded settings, so it waits for them.
  const finishing = !settings.data || update.isPending;

  return (
    <main className="grid h-full place-items-center overflow-y-auto p-3">
      <GlassPanel className="w-full max-w-[680px] px-6 pt-6 pb-2">
        <h1 className="text-h1">Welcome to Jobert's Vault</h1>
        <p className="pt-1 pb-3 text-muted-foreground">
          Set up your currency and accounts. You can change all of this later in
          Settings.
        </p>
        <Step n={1} title="Currency">
          {settings.data ? (
            <GeneralTab settings={settings.data} currencyOnly />
          ) : settings.isError ? (
            <ErrorState
              onRetry={() => settings.refetch()}
              pending={settings.isFetching}
            />
          ) : (
            loading
          )}
        </Step>
        <Step n={2} title="Your accounts">
          {accounts.data ? (
            <ul className="grid gap-1.5">
              {accounts.data.accounts
                .filter((a) => !a.archived)
                .map((a) => {
                  const Icon = ACCOUNT_ICONS[a.kind];
                  return (
                    <li key={a.id} className="flex items-center gap-2">
                      <Icon
                        aria-hidden
                        className="size-4 text-muted-foreground"
                      />
                      <span>{a.name}</span>
                      {existing?.includes(a.id) && (
                        <span className="text-small text-muted-foreground">
                          already added
                        </span>
                      )}
                    </li>
                  );
                })}
            </ul>
          ) : accounts.isError ? (
            <ErrorState
              onRetry={() => accounts.refetch()}
              pending={accounts.isFetching}
            />
          ) : (
            loading
          )}
          <div className="flex flex-wrap gap-1.5">
            {ADD.map((kind) => (
              <Button key={kind} size="sm" onClick={() => setAdding(kind)}>
                <Plus />
                {ACCOUNT_KINDS[kind]}
              </Button>
            ))}
          </div>
        </Step>
        <Step n={3}>
          <div className="flex items-center justify-between gap-3">
            <Button variant="primary" disabled={finishing} onClick={finish}>
              Start using the app
            </Button>
            <Button
              variant="ghost"
              className="text-primary"
              disabled={finishing}
              onClick={finish}
            >
              Set up later
            </Button>
          </div>
        </Step>
      </GlassPanel>
      {adding && (
        <AccountDialog kind={adding} onClose={() => setAdding(null)} />
      )}
    </main>
  );
}
