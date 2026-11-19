import { useQuery } from "@tanstack/react-query";
import { getVaultStatus } from "@/api/checkin";
import { queryKeys } from "@/lib/queryKeys";

export const useVaultStatus = () =>
  useQuery({ queryKey: queryKeys.vaultStatus, queryFn: getVaultStatus });
