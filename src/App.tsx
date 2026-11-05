import { RouterProvider } from "react-router";
import { AuroraBackground } from "./components/AuroraBackground";
import { StartupErrorScreen } from "./components/StartupErrorScreen";
import { Toaster } from "./components/ui/toast";
import { useStartupStatus } from "./hooks/useStartupStatus";
import { router } from "./routes";

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
        <StartupErrorScreen message={startup.error.message} />
      ) : status?.status === "ok" ? (
        <RouterProvider router={router} />
      ) : null}
    </>
  );
}
