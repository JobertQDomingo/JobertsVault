import { useQuery } from "@tanstack/react-query";
import {
  archiveLoan,
  createLoan,
  deleteLoan,
  listLoans,
  markLoanPaymentPaid,
  markLoanPaymentUnpaid,
  unarchiveLoan,
  updateLoan,
} from "@/api/loans";
import { queryKeys } from "@/lib/queryKeys";
import { useLedgerMutation } from "./useLedgerMutation";

/**
 * Quick-add asks for them only to mark a payment paid. `enabled`, not `skipToken`: an observer's
 * skipToken would become the shared query's fetcher, and the next refresh would fail.
 */
export const useLoans = (enabled = true) =>
  useQuery({ queryKey: queryKeys.loans, queryFn: listLoans, enabled });

// Ledger writes: a loan's amounts lock the decimals and put its account and category in use.
export const useCreateLoan = () => useLedgerMutation(createLoan);
export const useUpdateLoan = () => useLedgerMutation(updateLoan);
export const useArchiveLoan = () => useLedgerMutation(archiveLoan);
export const useUnarchiveLoan = () => useLedgerMutation(unarchiveLoan);
export const useDeleteLoan = () => useLedgerMutation(deleteLoan);
export const useMarkLoanPaymentPaid = () =>
  useLedgerMutation(markLoanPaymentPaid);
export const useMarkLoanPaymentUnpaid = () =>
  useLedgerMutation(markLoanPaymentUnpaid);
