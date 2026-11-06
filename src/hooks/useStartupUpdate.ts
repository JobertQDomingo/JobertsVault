import { useEffect, useRef, useState } from "react";
import { checkUpdate, installUpdate } from "@/api/system";
import { notify } from "@/components/ui/toast";
import { useSettings } from "./useSettings";

/** "Checking for updates…" appears only if the check takes longer than this (UI_FLOWS §F11). */
const SHOW_CHECKING_AFTER_MS = 150;

export type StartupUpdatePhase =
  | { step: "waiting" | "checking" | "done" }
  | { step: "updating"; version: string };

/**
 * Checks for and installs a newer version once, as the app starts, when automatic updates are on
 * (UPD-1). It's decided once, from the first settings read, so turning the switch on later never
 * starts an update in the middle of a session. A failed check opens the app with no error (UPD-2);
 * Rust has logged it. A failed install says why, then opens the app (UPD-4).
 */
export function useStartupUpdate(): StartupUpdatePhase {
  const settings = useSettings();
  // Decided once, from the first settings read. Unknown counts as off: "off" must mean the app
  // never connects by itself (ADR 0010).
  const [auto, setAuto] = useState<boolean | null>(null);
  if (auto === null && !settings.isPending) {
    setAuto(settings.data?.autoUpdateEnabled === true);
  }
  const [phase, setPhase] = useState<StartupUpdatePhase>({ step: "waiting" });
  const started = useRef(false);

  useEffect(() => {
    if (auto !== true || started.current) return;
    started.current = true;
    // No cleanup: the check runs once even when StrictMode remounts this in development.
    const slow = setTimeout(
      () => setPhase({ step: "checking" }),
      SHOW_CHECKING_AFTER_MS,
    );
    checkUpdate()
      .finally(() => clearTimeout(slow))
      .then(
        (version) => {
          if (version === null) return setPhase({ step: "done" });
          setPhase({ step: "updating", version });
          // Settles only if the installer didn't start: on success the app closes.
          return installUpdate()
            .catch((error: Error) => notify.error(error.message))
            .finally(() => setPhase({ step: "done" }));
        },
        () => setPhase({ step: "done" }),
      );
  }, [auto]);

  return auto === false ? { step: "done" } : phase;
}
