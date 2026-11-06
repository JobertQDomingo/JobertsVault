import type { UseQueryResult } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { useSearchParams } from "react-router";
import type { CategoryKind } from "@/bindings/CategoryKind";
import { ErrorState } from "@/components/ErrorState";
import { GlassPanel } from "@/components/GlassPanel";
import { IncomeExpenseChart } from "@/components/IncomeExpenseChart";
import { Loading, Skeleton } from "@/components/Loading";
import { PeriodSwitcher } from "@/components/PeriodSwitcher";
import { Segmented } from "@/components/ui/segmented";
import { useCategories } from "@/hooks/useCategories";
import { useBreakdown, useTimeseries } from "@/hooks/useReports";
import { usePeriodQuery } from "@/hooks/usePeriodQuery";
import { cn } from "@/lib/utils";
import { CategoryBreakdown } from "./CategoryBreakdown";

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

/** `/reports` (UI_FLOWS §8.1): the period bar and each report for that period. */
export function ReportsScreen() {
  const [query] = usePeriodQuery();
  const [params, setParams] = useSearchParams();
  // Kept in the URL like the period, so Back from a category's transactions returns to it.
  const kind: CategoryKind =
    params.get("breakdown") === "income" ? "income" : "expense";
  const categories = useCategories();
  return (
    <>
      <PeriodSwitcher />
      <Section
        title="Income vs expense"
        query={useTimeseries(query)}
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
              setParams((current) => {
                const updated = new URLSearchParams(current);
                if (next === "income") updated.set("breakdown", next);
                else updated.delete("breakdown");
                return updated;
              })
            }
          />
        }
        query={useBreakdown(query, kind)}
        also={[categories]}
        skeleton="h-[216px]"
      >
        {(breakdown) => (
          <CategoryBreakdown
            breakdown={breakdown}
            query={query}
            categories={
              new Map(categories.data?.categories.map((c) => [c.id, c]))
            }
          />
        )}
      </Section>
    </>
  );
}
