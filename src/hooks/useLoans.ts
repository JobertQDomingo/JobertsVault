import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import type { LoanDraft } from "@/bindings/LoanDraft";
import type { LoanRowAmount } from "@/bindings/LoanRowAmount";
import {
  archiveLoan,
  createLoan,
  deleteLoan,
  listLoans,
  markLoanPaymentPaid,
  markLoanPaymentUnpaid,
  previewLoan,
  totalLoanPayments,
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

/** Before you borrow (LOAN-9): the draft last tried, none until Try it. Saves nothing. */
export const useLoanPreview = (draft: LoanDraft | null) =>
  useQuery({
    queryKey: queryKeys.loanPreview(draft),
    queryFn: () => previewLoan(draft ?? { payments: [] }),
    enabled: draft !== null,
  });

/** The loan form's running total (F12), asked 250 ms after the rows stop changing, as Net pay is. */
export function useLoanRowsTotal(rows: LoanRowAmount[]) {
  const [asked, setAsked] = useState(rows);
  const key = JSON.stringify(rows);
  useEffect(() => {
    const timer = setTimeout(
      () => setAsked(JSON.parse(key) as LoanRowAmount[]),
      250,
    );
    return () => clearTimeout(timer);
  }, [key]);
  return useQuery({
    queryKey: queryKeys.loanRowsTotal(asked),
    queryFn: () => totalLoanPayments(asked),
    placeholderData: keepPreviousData,
  });
}

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
