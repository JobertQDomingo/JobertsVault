import { skipToken, useQuery, useQueryClient } from "@tanstack/react-query";
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

/** Asks nothing for `null` (an id in the URL that isn't a number). */
export const useAccount = (id: number | null) =>
  useQuery({
    queryKey: queryKeys.account(id ?? 0),
    queryFn: id === null ? skipToken : () => getAccount(id),
  });

export const useCreateAccount = () => useLedgerMutation(createAccount);
export const useUpdateAccount = () => useLedgerMutation(updateAccount);
export const useReorderAccounts = () => useLedgerMutation(reorderAccounts);
export const useArchiveAccount = () => useLedgerMutation(archiveAccount);
export const useUnarchiveAccount = () => useLedgerMutation(unarchiveAccount);
/**
 * Drops the deleted account's query before the ledger refresh, then runs `onDeleted` (still before
 * it), so a screen showing the account can leave first and the account isn't fetched again.
 */
export const useDeleteAccount = (onDeleted?: (id: number) => void) => {
  const client = useQueryClient();
  return useLedgerMutation(async (id: number) => {
    await deleteAccount(id);
    client.removeQueries({ queryKey: queryKeys.account(id) });
    onDeleted?.(id);
  });
};
