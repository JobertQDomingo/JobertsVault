import { PeriodSwitcher } from "@/components/PeriodSwitcher";
import { isFiltered, useTransactionFilter } from "@/hooks/useTransactionFilter";
import { FilterToolbar } from "./FilterToolbar";
import { TransactionList } from "./TransactionList";

/** `/transactions` (UI_FLOWS §3): the period bar, the filter toolbar and the list. */
export function TransactionsScreen() {
  const [filter, , clear] = useTransactionFilter();
  return (
    <>
      <PeriodSwitcher />
      <FilterToolbar />
      <TransactionList
        filter={filter}
        filtered={isFiltered(filter)}
        onClearFilters={clear}
      />
    </>
  );
}
