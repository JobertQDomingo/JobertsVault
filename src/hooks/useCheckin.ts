import { useQuery } from "@tanstack/react-query";
import { completeCheckin, getCheckin, getVaultStatus } from "@/api/checkin";
import { queryKeys } from "@/lib/queryKeys";
import { useLedgerMutation } from "./useLedgerMutation";

export const useCheckin = () =>
  useQuery({ queryKey: queryKeys.checkin, queryFn: getCheckin });

export const useCompleteCheckin = () => useLedgerMutation(completeCheckin);

export const useVaultStatus = () =>
  useQuery({ queryKey: queryKeys.vaultStatus, queryFn: getVaultStatus });
