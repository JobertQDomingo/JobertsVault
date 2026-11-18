import { useLayoutEffect } from "react";
import { useSettings } from "@/hooks/useSettings";

/**
 * Keeps the Reduce transparency switch (SET-8) as the `.reduce-transparency` class on <html>,
 * which globals.css's `transparency-off` variant shares with Windows' setting (DESIGN_SYSTEM
 * §2.4). Above the router, so screens outside the shell (/welcome, /check-in) follow it too. A
 * layout effect, so it lands before the first paint that shows the screens.
 */
export function FollowReduceTransparency() {
  const reduce = useSettings().data?.reduceTransparency ?? false;
  useLayoutEffect(() => {
    document.documentElement.classList.toggle("reduce-transparency", reduce);
  }, [reduce]);
  return null;
}
