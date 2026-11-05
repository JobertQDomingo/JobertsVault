import { keepPreviousData, useQuery } from "@tanstack/react-query";
import type { PeriodQuery } from "@/bindings/PeriodQuery";
import { getPeriod } from "@/api/period";
import { queryKeys } from "@/lib/queryKeys";

export const usePeriod = (query: PeriodQuery) =>
  useQuery({
    queryKey: queryKeys.period(query),
    queryFn: () => getPeriod(query),
    // The switcher keeps its label while the next period loads.
    placeholderData: keepPreviousData,
  });
