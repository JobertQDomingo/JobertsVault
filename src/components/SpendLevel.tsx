import { TriangleAlert } from "lucide-react";
import type { ReactNode } from "react";
import type { Account } from "@/bindings/Account";
import type { SpendLevel } from "@/bindings/SpendLevel";
import { cn } from "@/lib/utils";

type Reached = Exclude<SpendLevel, "none">;

/** Each level's place, so a level reached for the first time ranks higher (CC-11). */
export const LEVEL_RANK: Record<SpendLevel, number> = {
  none: 0,
  soft: 1,
  mid: 2,
  hard: 3,
};

const COLORS: Record<Reached, string> = {
  soft: "text-muted-foreground",
  mid: "text-warning",
  hard: "text-danger",
};

/** The card's amount for `level` (CC-10). */
export const levelAmount = (account: Account, level: Reached) =>
  ({ soft: account.spendSoft, mid: account.spendMid, hard: account.spendHard })[
    level
  ] ?? 0;

/**
 * The level this cycle's spending has reached (UI_FLOWS F16 step 5): its name in the muted
 * color, `--warning` or `--danger`, with an icon from mid up, so it never relies on color alone.
 * Nothing below soft. `children` replaces the "Soft level" text.
 */
export function SpendLevelTag({
  level,
  children,
}: {
  level: SpendLevel;
  children?: ReactNode;
}) {
  if (level === "none") return null;
  return (
    <span className={cn("inline-flex items-center gap-1", COLORS[level])}>
      {level !== "soft" && (
        <TriangleAlert aria-hidden className="size-3.5 shrink-0" />
      )}
      {children ?? `${level[0].toUpperCase()}${level.slice(1)} level`}
    </span>
  );
}
