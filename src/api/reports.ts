import type { Breakdown } from "@/bindings/Breakdown";
import type { CategoryKind } from "@/bindings/CategoryKind";
import type { Comparison } from "@/bindings/Comparison";
import type { ComparisonMode } from "@/bindings/ComparisonMode";
import type { PeriodQuery } from "@/bindings/PeriodQuery";
import type { Timeseries } from "@/bindings/Timeseries";
import { call } from "./client";

/** The income-vs-expense chart's bars for a period (RPT-1). */
export const getTimeseries = (query: PeriodQuery) =>
  call<Timeseries>("get_timeseries", { query });

/** A period's expenses or income by category, ranked, with the donut's slices (RPT-2). */
export const getCategoryBreakdown = (query: PeriodQuery, kind: CategoryKind) =>
  call<Breakdown>("get_category_breakdown", { query, kind });

/** The period next to the one before it (RPT-4). `mode` null means the setting (SET-7). */
export const getComparison = (
  query: PeriodQuery,
  mode: ComparisonMode | null,
) => call<Comparison>("get_comparison", { query, mode });
