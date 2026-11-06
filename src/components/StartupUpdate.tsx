import type { ReactNode } from "react";
import logo from "../../assets/brand/logo.svg";
import { useStartupUpdate } from "@/hooks/useStartupUpdate";
import { GlassPanel } from "./GlassPanel";

/** Shows the app once the startup update check is done (UPD-1, UPD-2). */
export function StartupUpdate({ children }: { children: ReactNode }) {
  const phase = useStartupUpdate();
  switch (phase.step) {
    case "waiting":
      return null;
    case "checking":
      return <Notice>Checking for updates…</Notice>;
    case "updating":
      return <Notice>Updating Jobert's Vault to {phase.version}…</Notice>;
    case "done":
      return children;
  }
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
