import { keepPreviousData, useQuery } from "@tanstack/react-query";
import type { CategoryKind } from "@/bindings/CategoryKind";
import type { ComparisonMode } from "@/bindings/ComparisonMode";
import type { PeriodQuery } from "@/bindings/PeriodQuery";
import {
  getCategoryBreakdown,
  getComparison,
  getTimeseries,
} from "@/api/reports";
import { queryKeys } from "@/lib/queryKeys";

// Switching period or account keeps the old report until the new one arrives, instead of
// blanking. `accountId` null means every account (RPT-5).

export const useTimeseries = (query: PeriodQuery, accountId: number | null) =>
  useQuery({
    queryKey: queryKeys.timeseries(query, accountId),
    queryFn: () => getTimeseries(query, accountId),
    placeholderData: keepPreviousData,
  });

export const useBreakdown = (
  query: PeriodQuery,
  kind: CategoryKind,
  accountId: number | null,
) =>
  useQuery({
    queryKey: queryKeys.breakdown(query, kind, accountId),
    queryFn: () => getCategoryBreakdown(query, kind, accountId),
    placeholderData: keepPreviousData,
  });

export const useComparison = (
  query: PeriodQuery,
  mode: ComparisonMode | null,
  accountId: number | null,
) =>
  useQuery({
    queryKey: queryKeys.comparison(query, mode, accountId),
    queryFn: () => getComparison(query, mode, accountId),
    placeholderData: keepPreviousData,
  });
