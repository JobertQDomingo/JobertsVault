import {
  CalendarRange,
  ChartPie,
  Flag,
  HandCoins,
  Hexagon,
  LayoutDashboard,
  Landmark,
  List,
  Repeat,
  Settings,
  Target,
  type LucideIcon,
} from "lucide-react";
import { Link, NavLink } from "react-router";
import logo from "../../assets/brand/logo.svg";
import { cn } from "@/lib/utils";
import { useVaultStatus } from "@/hooks/useCheckin";
import { GlassPanel } from "./GlassPanel";
import { LevelChip } from "./LevelBadge";

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
            "bg-[color-mix(in_srgb,var(--primary)_14%,transparent)] font-semibold text-primary shadow-[inset_0_0_0_1px_color-mix(in_srgb,var(--primary)_22%,transparent)] hover:bg-[color-mix(in_srgb,var(--primary)_14%,transparent)] hover:text-primary",
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
  // The Level screen's cached query; nothing shows while it loads (UI_FLOWS §1).
  const { data: status } = useVaultStatus();
  return (
    <GlassPanel className="flex flex-col px-2.5 py-3.5">
      <div className="flex items-center gap-2.5 px-2 pt-1 pb-4 font-bold tracking-[-0.01em]">
        <img src={logo} alt="" className="size-[30px]" />
        Jobert's Vault
        {status && (
          <Link
            to="/level"
            className="ml-auto rounded-full"
          >
            <LevelChip
              level={status.level}
              met={status.nextLevel.met}
              total={status.nextLevel.total}
              next={status.nextLevel.level}
            />
          </Link>
        )}
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
        <Item to="/level" icon={Hexagon} label="Level" />
        <div className="flex-1" />
        <hr className="mx-1 my-2 border-hairline" />
        <Item to="/settings" icon={Settings} label="Settings" shortcut="," />
      </nav>
    </GlassPanel>
  );
}
