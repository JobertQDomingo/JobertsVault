import {
  CalendarRange,
  ChartPie,
  Flag,
  HandCoins,
  LayoutDashboard,
  Landmark,
  List,
  Repeat,
  Settings,
  Target,
  type LucideIcon,
} from "lucide-react";
import { NavLink } from "react-router";
import logo from "../../assets/brand/logo.svg";
import { cn } from "@/lib/utils";
import { useVaultStatus } from "@/hooks/useCheckin";
import { GlassPanel } from "./GlassPanel";
import { LevelChip } from "./LevelBadge";

/** The current screen's tint: a nav item's, and the level chip's on /level (DESIGN_SYSTEM §9.2). */
const CURRENT =
  "bg-[color-mix(in_srgb,var(--primary)_14%,transparent)] shadow-[inset_0_0_0_1px_color-mix(in_srgb,var(--primary)_22%,transparent)]";

function Item({
  to,
  icon: Icon,
  label,
  shortcut,
}: {
  to: string;
  icon: LucideIcon;
  label: string;
  /** The key after Ctrl (UI_FLOWS §10); AppShell handles it. The planning and loan screens have none. */
  shortcut?: string;
}) {
  return (
    <NavLink
      to={to}
      title={shortcut ? `${label} (Ctrl+${shortcut})` : undefined}
      aria-keyshortcuts={shortcut ? `Control+${shortcut}` : undefined}
      // Only the dashboard needs an exact match: /accounts/5 still lights up Accounts.
      end={to === "/"}
      className={({ isActive }) =>
        cn(
          "flex h-[38px] items-center gap-2.5 rounded-control px-2.5 text-muted-foreground transition-colors duration-150 hover:bg-wash hover:text-foreground [&_svg]:size-5",
          isActive &&
            cn(
              CURRENT,
              "font-semibold text-primary hover:bg-[color-mix(in_srgb,var(--primary)_14%,transparent)] hover:text-primary",
            ),
        )
      }
    >
      <Icon aria-hidden />
      {label}
    </NavLink>
  );
}

/** The logo, every screen in UI_FLOWS §1 and Settings at the bottom. */
export function Sidebar() {
  // The Level screen's cached query. The chip is the only way to /level, so a placeholder stands in
  // while it loads or fails (UI_FLOWS §1, item 101).
  const { data: status } = useVaultStatus();
  return (
    <GlassPanel className="flex flex-col px-2.5 py-3.5">
      <div className="flex items-center gap-2.5 px-2 pt-1 pb-4 font-bold tracking-[-0.01em]">
        <img src={logo} alt="" className="size-[30px]" />
        Jobert's Vault
        <NavLink
          to="/level"
          title="Vault level"
          className={({ isActive }) =>
            cn("ml-auto rounded-full", isActive && CURRENT)
          }
        >
          <LevelChip status={status ?? undefined} />
        </NavLink>
      </div>
      <nav aria-label="Main" className="flex flex-1 flex-col gap-0.5">
        <Item to="/" icon={LayoutDashboard} label="Dashboard" shortcut="1" />
        <Item
          to="/transactions"
          icon={List}
          label="Transactions"
          shortcut="2"
        />
        <Item to="/accounts" icon={Landmark} label="Accounts" shortcut="3" />
        <Item to="/reports" icon={ChartPie} label="Reports" shortcut="4" />
        <Item to="/recurring" icon={Repeat} label="Recurring" />
        <Item to="/budgets" icon={Target} label="Budgets" />
        <Item to="/goals" icon={Flag} label="Goals" />
        <Item to="/debt-free" icon={HandCoins} label="Debt-free" />
        <Item to="/forecast" icon={CalendarRange} label="Forecast" />
        <div className="flex-1" />
        <hr className="mx-1 my-2 border-hairline" />
        <Item to="/settings" icon={Settings} label="Settings" shortcut="," />
      </nav>
    </GlassPanel>
  );
}
