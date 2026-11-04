import { useQuery } from "@tanstack/react-query";
import type { PeriodQuery } from "@/bindings/PeriodQuery";
import { getDashboard } from "@/api/dashboard";
import { queryKeys } from "@/lib/queryKeys";

export const useDashboard = (query: PeriodQuery) =>
  useQuery({
    queryKey: queryKeys.dashboard(query),
    queryFn: () => getDashboard(query),
  });
