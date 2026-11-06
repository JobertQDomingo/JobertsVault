import type { UseQueryResult } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { useSearchParams } from "react-router";
import type { CategoryKind } from "@/bindings/CategoryKind";
import { ErrorState } from "@/components/ErrorState";
import { GlassPanel } from "@/components/GlassPanel";
import { accountOptions } from "@/components/AccountRow";
import { IncomeExpenseChart } from "@/components/IncomeExpenseChart";
import { Loading, Skeleton } from "@/components/Loading";
import { PeriodSwitcher } from "@/components/PeriodSwitcher";
import { Combobox } from "@/components/ui/combobox";
import { Segmented } from "@/components/ui/segmented";
import { useAccounts } from "@/hooks/useAccounts";
import { useCategories } from "@/hooks/useCategories";
import { useBreakdown, useComparison, useTimeseries } from "@/hooks/useReports";
import { usePeriodQuery } from "@/hooks/usePeriodQuery";
import { useSettings } from "@/hooks/useSettings";
import { COMPARE } from "@/lib/comparison";
import { idParam } from "@/lib/records";
import { cn } from "@/lib/utils";
import { CategoryBreakdown } from "./CategoryBreakdown";
import { ComparisonTable } from "./ComparisonTable";

const KINDS: { value: CategoryKind; label: string }[] = [
  { value: "expense", label: "Expense" },
  { value: "income", label: "Income" },
];

/**
 * One report in its own panel, loading and failing on its own, along with any lists it needs
 * (`also`). While the next period loads, the old figures stay but dim (after 150 ms) and are marked
 * busy, as on the dashboard.
 */
function Section<T>({
  title,
  action,
  query,
  also = [],
  skeleton,
  children,
}: {
  title: string;
  action?: ReactNode;
  query: UseQueryResult<T>;
  also?: UseQueryResult[];
  /** The skeleton's height class, so it holds the loaded body's place. */
  skeleton: string;
  children: (data: T) => ReactNode;
}) {
  const failed = [query, ...also].filter((q) => q.isError);
  return (
    <GlassPanel className="grid gap-3 p-5">
      <div className="flex items-center justify-between gap-3">
        <h2 className="text-h3">{title}</h2>
        {action}
      </div>
      {failed.length ? (
        <ErrorState
          onRetry={() => failed.forEach((q) => q.refetch())}
          pending={failed.some((q) => q.isFetching)}
        />
      ) : query.data && also.every((q) => q.data) ? (
        <div
          aria-busy={query.isPlaceholderData}
          // The old figures can't be acted on: a click would open the new period's transactions.
          inert={query.isPlaceholderData}
          className={cn(
            "transition-opacity duration-150",
            query.isPlaceholderData && "opacity-60 delay-150",
          )}
        >
          {children(query.data)}
        </div>
      ) : (
        <Loading>
          <Skeleton className={cn("w-full", skeleton)} />
        </Loading>
      )}
    </GlassPanel>
  );
}

/**
 * `/reports` (UI_FLOWS §8.1): the period bar and Compare to, then each report for that period. The
 * breakdown's kind and Compare to are kept in the URL like the period, so Back from a category's
 * transactions returns to the same view.
 */
export function ReportsScreen() {
  const [query] = usePeriodQuery();
  const [params, setParams] = useSearchParams();
  const setParam = (key: string, value: string | null) =>
    setParams((current) => {
      const updated = new URLSearchParams(current);
      if (value === null) updated.delete(key);
      else updated.set(key, value);
      return updated;
    });
  const kind: CategoryKind =
    params.get("breakdown") === "income" ? "income" : "expense";
  // None means the setting (SET-7).
  const mode =
    COMPARE.find((m) => m.value === params.get("compare"))?.value ?? null;
  // None means every account (RPT-5).
  const accountId = idParam(params.get("account"));
  const accounts = useAccounts().data?.accounts;
  const settings = useSettings().data;
  const comparison = useComparison(query, mode, accountId);
  // A period that's over is compared in full whatever is chosen: the backend says so (RPT-4a).
  const over = comparison.data?.inProgress === false;
  const categories = useCategories();
  const byId = new Map(categories.data?.categories.map((c) => [c.id, c]));
  return (
    <>
      <div className="flex items-center gap-3">
        <PeriodSwitcher />
        <GlassPanel className="rounded-toolbar p-1.5">
          <Combobox
            aria-label="Account"
            // Shown only if the URL names an account the list doesn't have.
            placeholder="Account"
            className="w-48"
            options={accountOptions(accounts, accountId)}
            value={accountId === null ? "" : String(accountId)}
            onChange={(id) => setParam("account", id || null)}
          />
        </GlassPanel>
      </div>
      <Section
        title="Income vs expense"
        query={useTimeseries(query, accountId)}
        skeleton="h-[300px]"
      >
        {(series) => <IncomeExpenseChart series={series} height={272} />}
      </Section>
      <Section
        title="Category breakdown"
        action={
          <Segmented
            aria-label="Breakdown of"
            value={kind}
            options={KINDS}
            onChange={(next) =>
              setParam("breakdown", next === "income" ? next : null)
            }
          />
        }
        query={useBreakdown(query, kind, accountId)}
        also={[categories]}
        skeleton="h-[216px]"
      >
        {(breakdown) => (
          <CategoryBreakdown
            breakdown={breakdown}
            query={query}
            accountId={accountId}
            categories={byId}
          />
        )}
      </Section>
      <Section
        title="Period comparison"
        // It changes only this report, so it sits in its header (UI_FLOWS §8.1).
        action={
          <div
            className="flex items-center gap-2.5"
            title={
              over
                ? "A period that's over is always compared in full"
                : undefined
            }
          >
            <label htmlFor="compare-to" className="text-muted-foreground">
              Compare to
            </label>
            <Combobox
              id="compare-to"
              className="w-52"
              options={COMPARE}
              value={
                over
                  ? (comparison.data?.mode ?? null)
                  : (mode ?? settings?.comparisonMode ?? null)
              }
              disabled={over}
              onChange={(next) => setParam("compare", next)}
            />
          </div>
        }
        query={comparison}
        also={[categories]}
        skeleton="h-[240px]"
      >
        {(comparison) => (
          <ComparisonTable comparison={comparison} categories={byId} />
        )}
      </Section>
    </>
  );
}
