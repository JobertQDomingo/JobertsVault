import type { Dashboard } from "@/bindings/Dashboard";
import type { PeriodQuery } from "@/bindings/PeriodQuery";
import type { PeriodView } from "@/bindings/PeriodView";
import { call } from "./client";

/** A period's dates and the queries for ◀, ▶ and Today. The frontend never works dates out. */
export const getPeriod = (query: PeriodQuery) =>
  call<PeriodView>("get_period", { query });

/** Everything the dashboard shows, in one call (ARCHITECTURE §4.5). */
export const getDashboard = (query: PeriodQuery) =>
  call<Dashboard>("get_dashboard", { query });
