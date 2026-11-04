import { useQuery } from "@tanstack/react-query";
import type { PeriodQuery } from "@/bindings/PeriodQuery";
import { getDashboard, getPeriod } from "@/api/dashboard";
import { queryKeys } from "@/lib/queryKeys";

export const usePeriod = (query: PeriodQuery) =>
  useQuery({
    queryKey: queryKeys.period(query),
    queryFn: () => getPeriod(query),
  });

export const useDashboard = (query: PeriodQuery) =>
  useQuery({
    queryKey: queryKeys.dashboard(query),
    queryFn: () => getDashboard(query),
  });
