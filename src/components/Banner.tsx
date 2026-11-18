import { type LucideIcon, TriangleAlert } from "lucide-react";
import type { ReactNode } from "react";
import { cn } from "@/lib/utils";
import { GlassPanel } from "./GlassPanel";

const TONES = {
  warning: { glass: "glass-warning", icon: "text-warning" },
  info: { glass: "glass-info", icon: "text-primary" },
  danger: { glass: "glass-danger", icon: "text-danger" },
};

/**
 * A banner (DESIGN_SYSTEM §9.1): Level 1 glass tinted with its meaning color (`--warning`,
 * `--primary` for info, or `--danger-banner` at `--danger-banner-mix` for danger), an icon, the message, an optional second line in small
 * text and an optional action. Inside another panel it's a flat tint (§2.3).
 */
export function Banner({
  children,
  detail,
  action,
  tone = "warning",
  icon: Icon = TriangleAlert,
  className,
}: {
  children: ReactNode;
  detail?: ReactNode;
  action?: ReactNode;
  tone?: keyof typeof TONES;
  icon?: LucideIcon;
  className?: string;
}) {
  return (
    <GlassPanel
      role="status"
      className={cn(
        TONES[tone].glass,
        "flex items-center gap-3 rounded-toolbar py-2.5 pr-2.5 pl-4",
        className,
      )}
    >
      <Icon className={cn("size-4 shrink-0", TONES[tone].icon)} aria-hidden />
      <div className="min-w-0 flex-1">
        <p>{children}</p>
        {detail && <p className="text-small">{detail}</p>}
      </div>
      {action}
    </GlassPanel>
  );
}
