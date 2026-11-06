import type { UseQueryResult } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { ErrorState } from "@/components/ErrorState";
import { GlassPanel } from "@/components/GlassPanel";
import { IncomeExpenseChart } from "@/components/IncomeExpenseChart";
import { Loading, Skeleton } from "@/components/Loading";
import { PeriodSwitcher } from "@/components/PeriodSwitcher";
import { useTimeseries } from "@/hooks/useReports";
import { usePeriodQuery } from "@/hooks/usePeriodQuery";
import { cn } from "@/lib/utils";

/**
 * One report in its own panel, loading and failing on its own. While the next period loads, the
 * old figures stay but dim (after 150 ms) and are marked busy, as on the dashboard.
 */
function Section<T>({
  title,
  query,
  skeleton,
  children,
}: {
  title: string;
  query: UseQueryResult<T>;
  /** The skeleton's height class, so it holds the loaded body's place. */
  skeleton: string;
  children: (data: T) => ReactNode;
}) {
  return (
    <GlassPanel className="grid gap-3 p-5">
      <h2 className="text-h3">{title}</h2>
      {query.isError ? (
        <ErrorState
          onRetry={() => query.refetch()}
          pending={query.isFetching}
        />
      ) : query.data ? (
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
    </>
  );
}
