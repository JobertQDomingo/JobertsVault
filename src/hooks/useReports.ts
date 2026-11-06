import { keepPreviousData, useQuery } from "@tanstack/react-query";
import type { PeriodQuery } from "@/bindings/PeriodQuery";
import { getTimeseries } from "@/api/reports";
import { queryKeys } from "@/lib/queryKeys";

export const useTimeseries = (query: PeriodQuery) =>
  useQuery({
    queryKey: queryKeys.timeseries(query),
    queryFn: () => getTimeseries(query),
    // Switching period keeps the old chart until the new one arrives, instead of blanking.
    placeholderData: keepPreviousData,
  });
