import { useCallback, useMemo } from "react";
import { useSearchParams } from "react-router";
import type { TransactionFilter } from "@/bindings/TransactionFilter";
import type { TransactionKind } from "@/bindings/TransactionKind";
import { usePeriodQuery } from "./usePeriodQuery";

type Filters = Omit<TransactionFilter, "period">;

const KINDS: TransactionKind[] = ["income", "expense", "transfer"];
// Each filter's URL parameter (UI_FLOWS §3: all filters are saved in the URL).
const PARAMS = {
  kind: "type",
  accountId: "account",
  categoryId: "category",
  paymentMethodId: "method",
  search: "search",
} as const satisfies Record<keyof Filters, string>;

/** Whether any filter besides the period is set. */
export const isFiltered = (filter: TransactionFilter) =>
  (Object.keys(PARAMS) as (keyof Filters)[]).some(
    (key) => filter[key] !== null,
  );

/** A positive whole id, or null. */
const id = (text: string | null) =>
  text && /^\d+$/.test(text) && Number(text) > 0 ? Number(text) : null;

/**
 * The Transactions filters in the URL (TXN-4), with the period from `usePeriodQuery`. Anything
 * missing or malformed means "any". `change` keeps the period and the other filters; pass
 * `{ replace: true }` while typing, so each key doesn't add a history entry.
 */
export function useTransactionFilter() {
  const [period] = usePeriodQuery();
  const [params, setParams] = useSearchParams();
  const filter = useMemo<TransactionFilter>(() => {
    const kind = params.get(PARAMS.kind);
    return {
      period,
      kind: KINDS.find((k) => k === kind) ?? null,
      accountId: id(params.get(PARAMS.accountId)),
      categoryId: id(params.get(PARAMS.categoryId)),
      paymentMethodId: id(params.get(PARAMS.paymentMethodId)),
      search: params.get(PARAMS.search) || null,
    };
  }, [params, period]);

  const change = useCallback(
    (next: Partial<Filters>, options?: { replace?: boolean }) =>
      setParams((current) => {
        const updated = new URLSearchParams(current);
        for (const [key, value] of Object.entries(next)) {
          const param = PARAMS[key as keyof Filters];
          if (value === null || value === "") updated.delete(param);
          else updated.set(param, String(value));
        }
        return updated;
      }, options),
    [setParams],
  );
  const clear = useCallback(
    () =>
      setParams((current) => {
        const updated = new URLSearchParams(current);
        for (const param of Object.values(PARAMS)) updated.delete(param);
        return updated;
      }),
    [setParams],
  );
  return [filter, change, clear] as const;
}
