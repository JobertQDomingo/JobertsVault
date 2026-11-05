import { useQuery } from "@tanstack/react-query";
import { getStartupStatus } from "@/api/system";
import { queryKeys } from "@/lib/queryKeys";

/** Asked once, before anything else (ARCHITECTURE §6). It can't change while the app runs. */
export const useStartupStatus = () =>
  useQuery({
    queryKey: queryKeys.startup,
    queryFn: getStartupStatus,
    staleTime: Infinity,
    // An error is final too: it isn't asked again when the window gets focus.
    refetchOnWindowFocus: false,
  });
