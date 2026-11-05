import { cva, type VariantProps } from "class-variance-authority";
import { Slot } from "radix-ui";
import type { ComponentProps } from "react";
import { cn } from "@/lib/utils";

// DESIGN_SYSTEM §9.1: buttons are flat, never glass. Pressed, they shrink to 97% (§10).
const buttonVariants = cva(
  "inline-flex shrink-0 items-center justify-center gap-1.5 border border-transparent font-semibold whitespace-nowrap transition-[background-color,scale] duration-150 ease-out active:scale-[.97] disabled:pointer-events-none disabled:opacity-50 disabled:shadow-none [&_svg]:size-4",
  {
    variants: {
      variant: {
        primary:
          "bg-primary text-primary-foreground shadow-[inset_0_1px_0_rgb(255_255_255/.28),0_6px_18px_color-mix(in_srgb,var(--primary)_32%,transparent)] hover:bg-primary-hover",
        secondary:
          "border-field-edge bg-field text-foreground hover:bg-glass-strong",
        ghost: "text-foreground hover:bg-wash",
        destructive: "bg-danger text-danger-foreground",
      },
      size: {
        md: "h-9 rounded-control px-3.5",
        sm: "h-8 rounded-chip px-2.5 text-[13px]",
        icon: "size-8 rounded-chip",
      },
    },
    defaultVariants: { variant: "secondary", size: "md" },
  },
);

export function Button({
  variant,
  size,
  asChild = false,
  className,
  ...props
}: ComponentProps<"button"> &
  VariantProps<typeof buttonVariants> & { asChild?: boolean }) {
  const Component = asChild ? Slot.Root : "button";
  return (
    <Component
      // A button only submits when it says so, so Enter in a form saves on purpose.
      type={asChild ? undefined : "button"}
      className={cn(buttonVariants({ variant, size }), className)}
      {...props}
    />
  );
}
