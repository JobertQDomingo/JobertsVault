import { levelTitle } from "@/lib/vault";
import { cn } from "@/lib/utils";

// A pointy-top hexagon, 34.64 × 40 with 3 px rounded corners (DESIGN_SYSTEM §9.2).
const HEX =
  "M14.72 1.50Q17.32 0 19.92 1.50L32.04 8.50Q34.64 10 34.64 13.00L34.64 27.00Q34.64 30 32.04 31.50L19.92 38.50Q17.32 40 14.72 38.50L2.60 31.50Q0.00 30 0.00 27.00L0.00 13.00Q0.00 10 2.60 8.50Z";

/** The level's 40 px hexagon and "Level 4 · In the black" beside it (LVL-2). */
export function LevelBadge({ level }: { level: number }) {
  return (
    <div className="inline-flex items-center gap-3">
      <span
        aria-hidden
        className={cn(
          "relative grid h-10 w-[34.64px] shrink-0 place-items-center text-h2 tabular-nums",
          level === 0 ? "text-muted-foreground" : "text-primary-foreground",
        )}
      >
        <svg
          viewBox="0 0 34.64 40"
          className={cn(
            "absolute inset-0 size-full",
            level === 0 ? "fill-wash" : "fill-primary",
          )}
        >
          <path d={HEX} />
        </svg>
        <b className="relative font-semibold">{level}</b>
      </span>
      <span className="text-h2">{levelTitle(level)}</span>
    </div>
  );
}
