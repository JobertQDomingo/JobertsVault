import { levelTitle } from "@/lib/vault";
import { cn } from "@/lib/utils";

// A pointy-top hexagon, 34.64 × 40 with 3 px rounded corners (DESIGN_SYSTEM §9.2).
const HEX =
  "M14.72 1.50Q17.32 0 19.92 1.50L32.04 8.50Q34.64 10 34.64 13.00L34.64 27.00Q34.64 30 32.04 31.50L19.92 38.50Q17.32 40 14.72 38.50L2.60 31.50Q0.00 30 0.00 27.00L0.00 13.00Q0.00 10 2.60 8.50Z";

/** The hexagon with the level's number: 40 px, or the chip's 26 px when `small`. */
function LevelHex({
  level,
  small,
  levelUp,
}: {
  level: number;
  small?: boolean;
  levelUp?: boolean;
}) {
  return (
    <span
      aria-hidden
      className={cn(
        "relative grid shrink-0 place-items-center tabular-nums",
        levelUp && "animate-level-up",
        small ? "h-[26px] w-[22.52px] text-small" : "h-10 w-[34.64px] text-h2",
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
  );
}

/** The level's 40 px hexagon and "Level 4 · In the black" beside it (LVL-2). */
export function LevelBadge({
  level,
  levelUp,
}: {
  level: number;
  /** Plays the level-up moment (LVL-6, DESIGN_SYSTEM §10). */
  levelUp?: boolean;
}) {
  return (
    <div className="inline-flex items-center gap-3">
      <LevelHex level={level} levelUp={levelUp} />
      <span className="text-h2">{levelTitle(level)}</span>
    </div>
  );
}

const RING = 2 * Math.PI * 16;

/**
 * The sidebar's level chip (LVL-5): the 26 px hexagon inside a 34 px ring whose arc, from the top
 * and clockwise, is the share of the next level's conditions met.
 */
export function LevelChip({
  level,
  met,
  total,
  next,
}: {
  level: number;
  met: number;
  total: number;
  next: number;
}) {
  return (
    <span
      role="img"
      aria-label={`Level ${level}, ${met} of ${total} conditions met for level ${next}`}
      className="relative grid size-[34px] shrink-0 place-items-center"
    >
      <svg
        viewBox="0 0 34 34"
        aria-hidden
        className="absolute inset-0 size-full fill-none stroke-2"
      >
        <circle cx={17} cy={17} r={16} className="stroke-hairline" />
        {met > 0 && (
          <circle
            cx={17}
            cy={17}
            r={16}
            className="stroke-primary"
            strokeLinecap="round"
            strokeDasharray={`${((RING * met) / total).toFixed(2)} ${RING.toFixed(2)}`}
            transform="rotate(-90 17 17)"
          />
        )}
      </svg>
      <LevelHex level={level} small />
    </span>
  );
}
