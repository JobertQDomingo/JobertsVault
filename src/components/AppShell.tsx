import { useState } from "react";
import { Navigate, Outlet, useLocation, useNavigate } from "react-router";
import { useCheckin } from "@/hooks/useCheckin";
import { useProcessOnFocus } from "@/hooks/useProcessOnFocus";
import { useSettings } from "@/hooks/useSettings";
import { useShortcuts } from "@/hooks/useShortcuts";
import { QuickAddProvider, useQuickAdd } from "./QuickAddProvider";
import { Sidebar } from "./Sidebar";
import { TopBar } from "./TopBar";

/**
 * What `/check-in` gets from the guard: where Finish goes once the check-in is none (F17), and a
 * failed Finish's way past it (ADR 0014).
 */
export type GuardContext = {
  leaveTo: (path: string) => void;
  openAnyway: () => void;
};

/**
 * Above both the app shell and `/check-in` (ARCHITECTURE §5.4): the first launch goes to the setup
 * form (ONB-1, §6), then quick-add, Ctrl+N and the focus refresh work on every screen, the check-in
 * included. The settings take milliseconds; waiting for them means the dashboard never flashes
 * first. If they fail, the screens show their own errors.
 */
export function Root() {
  const settings = useSettings();
  if (settings.isPending) return null;
  if (settings.data?.onboardingCompleted === false)
    return <Navigate to="/welcome" replace />;
  return (
    <QuickAddProvider>
      <Guard />
    </QuickAddProvider>
  );
}

/**
 * The check-in guard (CHK-2): while one is due, every route goes to `/check-in`, and `/check-in`
 * goes to `/` once none is. A failed load opens the app normally, and the next focus or midnight
 * asks again. "Open the app anyway" lifts it until the next start: it's held here, never stored.
 */
function Guard() {
  const { add } = useQuickAdd();
  const checkin = useCheckin();
  const { pathname } = useLocation();
  const [anyway, setAnyway] = useState(false);
  const [exit, setExit] = useState("/");
  useProcessOnFocus();
  useShortcuts({ "Ctrl+N": add });
  const onCheckin = pathname === "/check-in";
  const context: GuardContext = {
    leaveTo: setExit,
    openAnyway: () => {
      setExit("/");
      setAnyway(true);
    },
  };
  // Until the first answer a guarded route shows nothing, so the dashboard never flashes first.
  if (checkin.isPending) return onCheckin ? <Outlet context={context} /> : null;
  const due = !!checkin.data && !checkin.isError && !anyway;
  if (due !== onCheckin)
    return <Navigate to={due ? "/check-in" : exit} replace />;
  return <Outlet context={context} />;
}

/**
 * The fixed 1440 × 900 layout (DESIGN_SYSTEM §5): the 228 px sidebar and the top bar stay put,
 * 12 px apart and from the window edge, and only the content under the top bar scrolls.
 */
export function AppShell() {
  const navigate = useNavigate();
  // The shortcuts that work everywhere but the check-in (UI_FLOWS §10). Ctrl+N is the Root's, and
  // the period ones live in PeriodSwitcher.
  useShortcuts({
    // The search box on Transactions (account details has none: UI_FLOWS §5.3 shows only the list).
    "Ctrl+F": () =>
      document
        .querySelector<HTMLInputElement>("main input[type=search]")
        ?.focus(),
    "Ctrl+1": () => navigate("/"),
    "Ctrl+2": () => navigate("/transactions"),
    "Ctrl+3": () => navigate("/accounts"),
    "Ctrl+4": () => navigate("/reports"),
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
