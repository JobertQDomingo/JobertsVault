import { TriangleAlert } from "lucide-react";
import type { ReactNode } from "react";
import { cn } from "@/lib/utils";
import { GlassPanel } from "./GlassPanel";

/**
 * A warning banner (DESIGN_SYSTEM §9.1): Level 1 glass tinted with `--warning`, an icon, the
 * message and an optional action. Inside another panel it's a flat tint (§2.3).
 */
export function Banner({
  children,
  action,
  className,
}: {
  children: ReactNode;
  action?: ReactNode;
  className?: string;
}) {
  return (
    <GlassPanel
      role="status"
      className={cn(
        "glass-warning flex items-center gap-3 rounded-toolbar py-2.5 pr-2.5 pl-4",
        className,
      )}
    >
      <TriangleAlert className="size-4 shrink-0 text-warning" aria-hidden />
      <p className="min-w-0 flex-1">{children}</p>
      {action}
    </GlassPanel>
  );
}
