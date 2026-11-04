import { useQuery } from "@tanstack/react-query";
import {
  archiveAccount,
  createAccount,
  deleteAccount,
  getAccount,
  listAccounts,
  reorderAccounts,
  unarchiveAccount,
  updateAccount,
} from "@/api/accounts";
import { queryKeys } from "@/lib/queryKeys";
import { useLedgerMutation } from "./useLedgerMutation";

export const useAccounts = () =>
  useQuery({ queryKey: queryKeys.accounts, queryFn: listAccounts });

export const useAccount = (id: number) =>
  useQuery({ queryKey: queryKeys.account(id), queryFn: () => getAccount(id) });

export const useCreateAccount = () => useLedgerMutation(createAccount);
export const useUpdateAccount = () => useLedgerMutation(updateAccount);
export const useReorderAccounts = () => useLedgerMutation(reorderAccounts);
export const useArchiveAccount = () => useLedgerMutation(archiveAccount);
export const useUnarchiveAccount = () => useLedgerMutation(unarchiveAccount);
export const useDeleteAccount = () => useLedgerMutation(deleteAccount);
