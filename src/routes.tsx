import { createBrowserRouter, Navigate, type RouteObject } from "react-router";
import { AppShell } from "./components/AppShell";
import { AccountsScreen } from "./features/accounts/AccountsScreen";
import { DashboardScreen } from "./features/dashboard/DashboardScreen";
import { TransactionsScreen } from "./features/transactions/TransactionsScreen";

// The Phase 1 routes (ARCHITECTURE §5.4). Each screen's `handle.title` is the top bar's title.
// shortcut: the screens are placeholders, ROADMAP 1.9 builds them.
export const routes: RouteObject[] = [
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
      { path: "accounts/:id", element: null, handle: { title: "Accounts" } },
      { path: "settings", element: null, handle: { title: "Settings" } },
      { path: "*", element: <Navigate to="/" replace /> },
    ],
  },
];

export const router = createBrowserRouter(routes);
