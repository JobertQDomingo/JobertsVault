import { useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";
import { processRecurring } from "@/api/recurring";
import { queryKeys } from "@/lib/queryKeys";

/**
 * The date may have changed while the window was in the background, so a focus posts or queues
 * the recurring dates now due (REC-4, ARCHITECTURE §8). On the same day it finds nothing: startup
 * and every rule save already processed up to today.
 */
export function useProcessOnFocus() {
  const client = useQueryClient();
  useEffect(() => {
    const process = async () => {
      // A failure is in the error log. The rules before it may have posted, so refresh anyway.
      const processed = await processRecurring().catch(() => 1);
      if (processed > 0)
        await client.invalidateQueries({ queryKey: queryKeys.ledger });
    };
    const onFocus = () => void process();
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, [client]);
}
