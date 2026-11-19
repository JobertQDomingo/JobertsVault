import { useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";
import { processRecurring } from "@/api/recurring";
import { queryKeys } from "@/lib/queryKeys";

/** Milliseconds from `now` to just after the next local midnight. */
export const toMidnight = (now: Date) =>
  new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1).getTime() -
  now.getTime() +
  1000;

/**
 * The date may have changed while the window was in the background, so a focus posts or queues
 * the recurring dates now due (REC-4, ARCHITECTURE §8), then asks for the check-in again, which
 * may have come due (CHK-2, §5.4). The same runs at local midnight while the app is open
 * (ADR 0014). On the same day it finds nothing: startup and every rule save already processed
 * up to today.
 */
export function useProcessOnFocus() {
  const client = useQueryClient();
  useEffect(() => {
    const process = async () => {
      // A failure is in the error log. The rules before it may have posted, so refresh anyway.
      const processed = await processRecurring().catch(() => 1);
      // The ledger includes the check-in; on a quiet day only the check-in is asked for again.
      if (processed > 0)
        await client.invalidateQueries({ queryKey: queryKeys.ledger });
      else await client.invalidateQueries({ queryKey: queryKeys.checkin });
    };
    const onFocus = () => void process();
    window.addEventListener("focus", onFocus);
    // shortcut: a timer that slept through midnight fires late; the focus on waking covers it.
    let timer: ReturnType<typeof setTimeout>;
    const atMidnight = () => {
      timer = setTimeout(() => {
        void process();
        atMidnight();
      }, toMidnight(new Date()));
    };
    atMidnight();
    return () => {
      window.removeEventListener("focus", onFocus);
      clearTimeout(timer);
    };
  }, [client]);
}
