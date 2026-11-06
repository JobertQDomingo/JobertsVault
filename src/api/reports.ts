import type { Breakdown } from "@/bindings/Breakdown";
import type { CategoryKind } from "@/bindings/CategoryKind";
import type { PeriodQuery } from "@/bindings/PeriodQuery";
import type { Timeseries } from "@/bindings/Timeseries";
import { call } from "./client";

/** The income-vs-expense chart's bars for a period (RPT-1). */
export const getTimeseries = (query: PeriodQuery) =>
  call<Timeseries>("get_timeseries", { query });

/** A period's expenses or income by category, ranked, with the donut's slices (RPT-2). */
export const getCategoryBreakdown = (query: PeriodQuery, kind: CategoryKind) =>
  call<Breakdown>("get_category_breakdown", { query, kind });
