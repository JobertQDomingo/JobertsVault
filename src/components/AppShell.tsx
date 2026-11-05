import { Outlet, useNavigate } from "react-router";
import { useShortcuts } from "@/hooks/useShortcuts";
import { QuickAddProvider, useQuickAdd } from "./QuickAddProvider";
import { Sidebar } from "./Sidebar";
import { TopBar } from "./TopBar";

/**
 * The fixed 1440 × 900 layout (DESIGN_SYSTEM §5): the 228 px sidebar and the top bar stay put,
 * 12 px apart and from the window edge, and only the content under the top bar scrolls.
 */
export function AppShell() {
  return (
    <QuickAddProvider>
      <Layout />
    </QuickAddProvider>
  );
}

function Layout() {
  const navigate = useNavigate();
  const openQuickAdd = useQuickAdd();
  // The shortcuts that work everywhere (UI_FLOWS §10). The period ones live in PeriodSwitcher.
  useShortcuts({
    "Ctrl+N": openQuickAdd,
    // The screen's search box, on Transactions and an account's details.
    "Ctrl+F": () =>
      document
        .querySelector<HTMLInputElement>("main input[type=search]")
        ?.focus(),
    "Ctrl+1": () => navigate("/"),
    "Ctrl+2": () => navigate("/transactions"),
    "Ctrl+3": () => navigate("/accounts"),
    "Ctrl+,": () => navigate("/settings"),
  });
  return (
    <div className="grid h-full grid-cols-[228px_minmax(0,1fr)] gap-3 p-3">
      <Sidebar />
      <div className="flex min-h-0 min-w-0 flex-col gap-3">
        <TopBar />
        {/* Spans the 12 px around it, so the panels' shadows aren't cut off at its edges. */}
        <main className="-m-3 grid min-h-0 flex-1 content-start gap-3 overflow-y-auto p-3">
          <Outlet />
        </main>
      </div>
    </div>
  );
}
