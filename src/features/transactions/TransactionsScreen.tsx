import { PeriodSwitcher } from "@/components/PeriodSwitcher";
import { useSettings } from "@/hooks/useSettings";
import { isFiltered, useTransactionFilter } from "@/hooks/useTransactionFilter";
import { FilterToolbar } from "./FilterToolbar";
import { TransactionList } from "./TransactionList";

/** `/transactions` (UI_FLOWS §3): the period bar, the filter toolbar and the list. */
export function TransactionsScreen() {
  const [filter, , clear] = useTransactionFilter();
  const { data: settings } = useSettings();
  // With the payment method field off (SET-6), its filter is hidden and ignored.
  const shown = settings?.showPaymentMethod
    ? filter
    : { ...filter, paymentMethodId: null };
  return (
    <>
      <PeriodSwitcher />
      <FilterToolbar />
      <TransactionList
        filter={shown}
        filtered={isFiltered(shown)}
        onClearFilters={clear}
      />
    </>
  );
}
