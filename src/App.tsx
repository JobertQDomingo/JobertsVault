import { AuroraBackground } from "./components/AuroraBackground";
import { GlassPanel } from "./components/GlassPanel";
import { StartupErrorScreen } from "./components/StartupErrorScreen";
import { Toaster } from "./components/ui/toast";
import { useStartupStatus } from "./hooks/useStartupStatus";

export default function App() {
  const startup = useStartupStatus();
  const status = startup.data;
  return (
    <>
      <AuroraBackground />
      <Toaster />
      {status?.status === "failed" ? (
        <StartupErrorScreen
          message={status.message}
          databasePath={status.databasePath}
        />
      ) : startup.isError ? (
        // Never a blank window, even if the status itself can't be read.
        <StartupErrorScreen message={startup.error.message} databasePath="" />
      ) : status?.status === "ok" ? (
        <main className="flex h-full items-center justify-center">
          <GlassPanel className="px-8 py-6">
            <h1 className="text-h1">Jobert's Vault</h1>
          </GlassPanel>
        </main>
      ) : null}
    </>
  );
}
