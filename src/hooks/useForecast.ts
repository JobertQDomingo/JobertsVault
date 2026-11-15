import { keepPreviousData, useQuery } from "@tanstack/react-query";
import type { ForecastGrain } from "@/bindings/ForecastGrain";
import { getForecast } from "@/api/forecast";
import { queryKeys } from "@/lib/queryKeys";

export const useForecast = (grain: ForecastGrain) =>
  useQuery({
    queryKey: queryKeys.forecast(grain),
    queryFn: () => getForecast(grain),
    // The other view keeps the card and the switch on screen while it loads.
    placeholderData: keepPreviousData,
  });
