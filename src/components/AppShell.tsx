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
  const { add } = useQuickAdd();
  // The shortcuts that work everywhere (UI_FLOWS §10). The period ones live in PeriodSwitcher.
  useShortcuts({
    "Ctrl+N": add,
    // The screen's search box, on Transactions and an account's details.
    "Ctrl+F": () =>
      document
        .querySelector<HTMLInputElement>("main input[type=search]")
        ?.focus(),
    "Ctrl+1": () => navigate("/"),
    "Ctrl+2": () => navigate("/transactions"),
    "Ctrl+3": () => navigate("/accounts"),
    "Ctrl+,": () => navigate("/settings"),
    // These mean the period (PeriodSwitcher handles them). Claimed here too, so on a screen
    // without one they do nothing instead of WebView2's Back and Forward.
    "Alt+ArrowLeft": () => {},
    "Alt+ArrowRight": () => {},
    "Alt+Home": () => {},
  });
  return (
    <div className="grid h-full grid-cols-[228px_minmax(0,1fr)] gap-3 p-3">
      <Sidebar />
      <div className="flex min-h-0 min-w-0 flex-col gap-3">
        <TopBar />
        {/* Spans the 12 px around it, so the panels' shadows aren't cut off at its edges. A
            column, so a screen's list panel can take the height left and scroll inside itself. */}
        <main className="-m-3 flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto p-3">
          <Outlet />
        </main>
      </div>
    </div>
  );
}
