import { useQuery } from "@tanstack/react-query";
import { useEffect, useState, type ReactNode } from "react";
import logo from "../../assets/brand/logo.svg";
import { checkUpdate, installUpdate } from "@/api/system";
import { useSettings } from "@/hooks/useSettings";
import { queryKeys } from "@/lib/queryKeys";
import { GlassPanel } from "./GlassPanel";
import { notify } from "./ui/toast";

/** "Checking for updates…" appears only if the check takes longer than this (UI_FLOWS §F11). */
const SHOW_CHECKING_AFTER_MS = 150;

// Each runs once per start: never again on focus, and never retried.
const ONCE = {
  staleTime: Infinity,
  gcTime: Infinity,
  retry: false,
  refetchOnWindowFocus: false,
} as const;

/**
 * Before the app opens, checks for and installs a newer version when automatic updates are on
 * (UPD-1). A failed check, or no internet, opens the app as usual with no error (UPD-2). A failed
 * install says why, then opens the app (UPD-4).
 */
export function StartupUpdate({ children }: { children: ReactNode }) {
  const settings = useSettings();
  const auto = settings.data?.autoUpdateEnabled === true;
  const check = useQuery({
    queryKey: queryKeys.startupUpdateCheck,
    queryFn: checkUpdate,
    enabled: auto,
    ...ONCE,
  });
  const version = check.data ?? null;
  const install = useQuery({
    queryKey: queryKeys.startupUpdateInstall,
    // Resolves only if it failed to start: on success the app closes and reopens.
    queryFn: installUpdate,
    enabled: version !== null,
    ...ONCE,
  });

  const installError = install.error?.message;
  useEffect(() => {
    if (installError) notify.error(installError);
  }, [installError]);

  const [slow, setSlow] = useState(false);
  useEffect(() => {
    const timer = setTimeout(() => setSlow(true), SHOW_CHECKING_AFTER_MS);
    return () => clearTimeout(timer);
  }, []);

  // The settings are read from this computer, so this wait is short.
  if (settings.isPending) return null;
  if (auto && check.isPending) {
    return slow ? <Notice>Checking for updates…</Notice> : null;
  }
  if (version !== null && !install.isError) {
    return <Notice>Updating Jobert's Vault to {version}…</Notice>;
  }
  return children;
}

/** A centered glass panel with the logo, in place of the app (UI_FLOWS §9). */
function Notice({ children }: { children: ReactNode }) {
  return (
    <main className="grid h-full place-items-center p-3">
      <GlassPanel className="flex items-center gap-3 px-8 py-6" role="status">
        <img src={logo} alt="" className="size-10" />
        <p className="text-h2">{children}</p>
      </GlassPanel>
    </main>
  );
}
