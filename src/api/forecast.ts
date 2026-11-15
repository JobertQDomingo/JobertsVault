import type { Forecast } from "@/bindings/Forecast";
import type { ForecastGrain } from "@/bindings/ForecastGrain";
import { call } from "./client";

/** Safe to spend and the 12-month forecast, by pay window or month (SAFE-1 to SAFE-7). */
export const getForecast = (grain: ForecastGrain) =>
  call<Forecast>("get_forecast", { grain });
