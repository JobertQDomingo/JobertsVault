import { createBrowserRouter, Navigate, type RouteObject } from "react-router";
import { AppShell } from "./components/AppShell";
import { AccountDetailsScreen } from "./features/accounts/AccountDetailsScreen";
import { AccountsScreen } from "./features/accounts/AccountsScreen";
import { SettingsScreen } from "./features/settings/SettingsScreen";
import { WelcomeScreen } from "./features/onboarding/WelcomeScreen";
import { DashboardScreen } from "./features/dashboard/DashboardScreen";
import { TransactionsScreen } from "./features/transactions/TransactionsScreen";

// The Phase 1 routes (ARCHITECTURE §5.4). Each screen's `handle.title` is the top bar's title.
// shortcut: the screens are placeholders, ROADMAP 1.9 builds them.
export const routes: RouteObject[] = [
  // The first-launch form fills the window, without the sidebar (ONB-1).
  { path: "welcome", element: <WelcomeScreen /> },
  {
    element: <AppShell />,
    children: [
      {
        index: true,
        element: <DashboardScreen />,
        handle: { title: "Dashboard" },
      },
      {
        path: "transactions",
        element: <TransactionsScreen />,
        handle: { title: "Transactions" },
      },
      {
        path: "accounts",
        element: <AccountsScreen />,
        handle: { title: "Accounts" },
      },
      {
        path: "accounts/:id",
        element: <AccountDetailsScreen />,
        handle: { title: "Accounts" },
      },
      {
        path: "settings",
        element: <SettingsScreen />,
        handle: { title: "Settings" },
      },
      { path: "*", element: <Navigate to="/" replace /> },
    ],
  },
];

export const router = createBrowserRouter(routes);
