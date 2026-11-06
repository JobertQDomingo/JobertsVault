import { useQuery } from "@tanstack/react-query";
import { getAppVersion } from "@/api/system";
import { queryKeys } from "@/lib/queryKeys";

/** The installed version. It can't change while the app runs. */
export const useAppVersion = () =>
  useQuery({
    queryKey: queryKeys.appVersion,
    queryFn: getAppVersion,
    staleTime: Infinity,
  });
