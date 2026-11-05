import type { ComponentProps } from "react";
import { cn } from "@/lib/utils";

/**
 * A frosted panel (DESIGN_SYSTEM §2.1): Level 1 for cards, the sidebar and toolbars, or with
 * `raised`, Level 2 for dialogs, menus and toasts. Text belongs on one of these, never on the aurora.
 */
export function GlassPanel({
  raised = false,
  className,
  ...props
}: ComponentProps<"div"> & { raised?: boolean }) {
  return (
    <div
      className={cn(
        raised ? "glass-raised rounded-dialog" : "glass rounded-panel",
        className,
      )}
      {...props}
    />
  );
}
