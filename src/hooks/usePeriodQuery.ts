import { useCallback, useMemo } from "react";
import { useSearchParams } from "react-router";
import type { PeriodQuery } from "@/bindings/PeriodQuery";
import { isIsoDate } from "@/lib/dates";

const KINDS = ["day", "week", "month", "year"] as const;
const KEYS = ["period", "anchor", "start", "end"];

/** The period in the URL; anything missing or malformed means this month. */
function readPeriod(params: URLSearchParams): PeriodQuery {
  const kind = params.get("period");
  if (kind === "custom") {
    const start = params.get("start") ?? "";
    const end = params.get("end") ?? "";
    // ISO dates compare correctly as text.
    if (isIsoDate(start) && isIsoDate(end) && start <= end)
      return { kind, start, end };
  } else if (KINDS.some((k) => k === kind)) {
    const anchor = params.get("anchor") ?? "";
    return {
      kind: kind as (typeof KINDS)[number],
      anchor: isIsoDate(anchor) ? anchor : null,
    };
  }
  return { kind: "month", anchor: null };
}

/** Writes `query` into `params` the way the URL keeps it, replacing any period already there. */
export function writePeriod(params: URLSearchParams, query: PeriodQuery) {
  for (const key of KEYS) params.delete(key);
  params.set("period", query.kind);
  if (query.kind === "custom") {
    params.set("start", query.start);
    params.set("end", query.end);
  } else if (query.anchor) params.set("anchor", query.anchor);
  return params;
}

/**
 * The selected period, saved in the URL so the back button and reloads keep it (ARCHITECTURE
 * §5.1): `?period=month&anchor=2026-10-26` or `?period=custom&start=…&end=…`. Other parameters,
 * such as the Transactions filters, are left alone.
 */
export function usePeriodQuery() {
  const [params, setParams] = useSearchParams();
  const query = useMemo(() => readPeriod(params), [params]);
  const setQuery = useCallback(
    (next: PeriodQuery) =>
      setParams((current) => writePeriod(new URLSearchParams(current), next)),
    [setParams],
  );
  return [query, setQuery] as const;
}
