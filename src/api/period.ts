import type { PeriodQuery } from "@/bindings/PeriodQuery";
import type { PeriodView } from "@/bindings/PeriodView";
import { call } from "./client";

/** A period's dates and the queries for ◀, ▶ and Today. The frontend never works dates out. */
export const getPeriod = (query: PeriodQuery) =>
  call<PeriodView>("get_period", { query });
