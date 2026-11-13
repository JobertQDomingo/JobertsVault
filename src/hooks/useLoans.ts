import { useQuery } from "@tanstack/react-query";
import {
  archiveLoan,
  createLoan,
  deleteLoan,
  listLoans,
  unarchiveLoan,
  updateLoan,
} from "@/api/loans";
import { queryKeys } from "@/lib/queryKeys";
import { useLedgerMutation } from "./useLedgerMutation";

export const useLoans = () =>
  useQuery({ queryKey: queryKeys.loans, queryFn: listLoans });

// Ledger writes: a loan's amounts lock the decimals and put its account and category in use.
export const useCreateLoan = () => useLedgerMutation(createLoan);
export const useUpdateLoan = () => useLedgerMutation(updateLoan);
export const useArchiveLoan = () => useLedgerMutation(archiveLoan);
export const useUnarchiveLoan = () => useLedgerMutation(unarchiveLoan);
export const useDeleteLoan = () => useLedgerMutation(deleteLoan);
