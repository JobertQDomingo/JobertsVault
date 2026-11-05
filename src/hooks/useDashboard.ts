import { keepPreviousData, useQuery } from "@tanstack/react-query";
import type { PeriodQuery } from "@/bindings/PeriodQuery";
import { getDashboard } from "@/api/dashboard";
import { queryKeys } from "@/lib/queryKeys";

export const useDashboard = (query: PeriodQuery) =>
  useQuery({
    queryKey: queryKeys.dashboard(query),
    queryFn: () => getDashboard(query),
    // Switching period keeps the old numbers until the new ones arrive, instead of blanking.
    placeholderData: keepPreviousData,
  });
