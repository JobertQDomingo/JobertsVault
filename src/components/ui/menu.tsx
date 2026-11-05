import { DropdownMenu } from "radix-ui";
import type { ComponentProps } from "react";
import { cn } from "@/lib/utils";

// A ⋯ menu (DESIGN_SYSTEM §9.1: menus are Level 2). Radix gives the arrow keys, Esc and focus return.
export const Menu = DropdownMenu.Root;
export const MenuTrigger = DropdownMenu.Trigger;

export function MenuContent({
  className,
  ...props
}: ComponentProps<typeof DropdownMenu.Content>) {
  return (
    <DropdownMenu.Portal>
      <DropdownMenu.Content
        align="end"
        sideOffset={6}
        className={cn(
          "glass-raised z-50 min-w-40 rounded-toolbar p-1",
          className,
        )}
        {...props}
      />
    </DropdownMenu.Portal>
  );
}

export function MenuItem({
  destructive = false,
  className,
  ...props
}: ComponentProps<typeof DropdownMenu.Item> & { destructive?: boolean }) {
  return (
    <DropdownMenu.Item
      className={cn(
        "flex cursor-pointer items-center gap-2 rounded-chip px-2 py-1.5 outline-none select-none data-[disabled]:opacity-50 data-[highlighted]:bg-wash [&_svg]:size-4",
        destructive && "text-danger",
        className,
      )}
      {...props}
    />
  );
}
