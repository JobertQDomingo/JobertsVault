import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

/** Inside a panel: a 24 px icon, one line of text and one main action (DESIGN_SYSTEM §9.1). */
export function EmptyState({
  icon: Icon,
  message,
  action,
  className,
}: {
  icon: LucideIcon;
  message: ReactNode;
  action?: ReactNode;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "grid justify-items-center gap-2.5 px-4 py-8 text-center",
        className,
      )}
    >
      <Icon aria-hidden className="size-6 text-muted-foreground" />
      <p>{message}</p>
      {action}
    </div>
  );
}
