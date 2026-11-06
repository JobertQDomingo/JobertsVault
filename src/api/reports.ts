import type { PeriodQuery } from "@/bindings/PeriodQuery";
import type { Timeseries } from "@/bindings/Timeseries";
import { call } from "./client";

/** The income-vs-expense chart's bars for a period (RPT-1). */
export const getTimeseries = (query: PeriodQuery) =>
  call<Timeseries>("get_timeseries", { query });
