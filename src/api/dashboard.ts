import type { Dashboard } from "@/bindings/Dashboard";
import type { PeriodQuery } from "@/bindings/PeriodQuery";
import { call } from "./client";

/** Everything the dashboard shows, in one call (ARCHITECTURE §4.5). */
export const getDashboard = (query: PeriodQuery) =>
  call<Dashboard>("get_dashboard", { query });
