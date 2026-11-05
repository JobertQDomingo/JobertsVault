import { useEffect, useState, type ReactNode } from "react";
import { cn } from "@/lib/utils";

/** A flat `--wash` block, with no shimmer (DESIGN_SYSTEM §9.1). */
export function Skeleton({ className }: { className?: string }) {
  return <div className={cn("rounded-[6px] bg-wash", className)} />;
}

/**
 * Shows `children` (a skeleton layout) only once loading has taken 150 ms, so a fast load never
 * flashes one (UI_FLOWS §9). Screen readers hear "Loading" straight away.
 */
export function Loading({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) {
  const [late, setLate] = useState(false);
  useEffect(() => {
    const timer = setTimeout(() => setLate(true), 150);
    return () => clearTimeout(timer);
  }, []);
  return (
    <div role="status" aria-label="Loading" className={className}>
      {late && children}
    </div>
  );
}
