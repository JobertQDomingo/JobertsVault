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
      setParams((current) => {
        const updated = new URLSearchParams(current);
        for (const key of KEYS) updated.delete(key);
        updated.set("period", next.kind);
        if (next.kind === "custom") {
          updated.set("start", next.start);
          updated.set("end", next.end);
        } else if (next.anchor) updated.set("anchor", next.anchor);
        return updated;
      }),
    [setParams],
  );
  return [query, setQuery] as const;
}
