import { keepPreviousData, useQuery } from "@tanstack/react-query";
import type { CategoryKind } from "@/bindings/CategoryKind";
import type { PeriodQuery } from "@/bindings/PeriodQuery";
import { getCategoryBreakdown, getTimeseries } from "@/api/reports";
import { queryKeys } from "@/lib/queryKeys";

// Switching period keeps the old report until the new one arrives, instead of blanking.

export const useTimeseries = (query: PeriodQuery) =>
  useQuery({
    queryKey: queryKeys.timeseries(query),
    queryFn: () => getTimeseries(query),
    placeholderData: keepPreviousData,
  });

export const useBreakdown = (query: PeriodQuery, kind: CategoryKind) =>
  useQuery({
    queryKey: queryKeys.breakdown(query, kind),
    queryFn: () => getCategoryBreakdown(query, kind),
    placeholderData: keepPreviousData,
  });
