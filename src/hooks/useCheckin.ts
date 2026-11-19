import { useQuery } from "@tanstack/react-query";
import { completeCheckin, getCheckin, getVaultStatus } from "@/api/checkin";
import type { FocusFields } from "@/bindings/FocusFields";
import { queryKeys } from "@/lib/queryKeys";
import { useLedgerMutation } from "./useLedgerMutation";

export const useCheckin = () =>
  useQuery({ queryKey: queryKeys.checkin, queryFn: getCheckin });

/**
 * Finish. `onSaved` runs as soon as it's saved, before the ledger refresh, so the screen can set
 * the check-in to none and move on before the guard sees it go (ARCHITECTURE §5.4).
 */
export const useCompleteCheckin = (onSaved: () => void) =>
  useLedgerMutation((focus: FocusFields) =>
    completeCheckin(focus).then((written) => {
      onSaved();
      return written;
    }),
  );

export const useVaultStatus = () =>
  useQuery({ queryKey: queryKeys.vaultStatus, queryFn: getVaultStatus });
