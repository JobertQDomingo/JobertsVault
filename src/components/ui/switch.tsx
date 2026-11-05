import { Switch as SwitchPrimitive } from "radix-ui";
import type { ComponentProps } from "react";
import { cn } from "@/lib/utils";

/** An on/off switch (the mockup's 34 × 20 track): `--wash` when off, solid accent when on. */
export function Switch({
  className,
  ...props
}: ComponentProps<typeof SwitchPrimitive.Root>) {
  return (
    <SwitchPrimitive.Root
      className={cn(
        "relative h-5 w-[34px] shrink-0 rounded-pill border border-field-edge bg-wash outline-none focus-visible:ring-3 focus-visible:ring-ring/40 disabled:opacity-50 data-[state=checked]:border-primary data-[state=checked]:bg-primary",
        className,
      )}
      {...props}
    >
      <SwitchPrimitive.Thumb className="block size-3.5 translate-x-0.5 rounded-full bg-muted-foreground transition-transform duration-150 data-[state=checked]:translate-x-4 data-[state=checked]:bg-primary-foreground motion-reduce:transition-none" />
    </SwitchPrimitive.Root>
  );
}
