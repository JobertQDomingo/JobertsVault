import { Check, Lock } from "lucide-react";
import { Tooltip } from "radix-ui";
import { cn } from "@/lib/utils";
import { CONDITIONS, LEVEL_CONDITIONS, levelTitle } from "@/lib/vault";

/**
 * The ten levels in a row (DESIGN_SYSTEM §9.2): reached ones filled with a check, the current one
 * ringed, the later ones hollow with a lock. Each node is a tab stop whose tooltip gives the
 * level's name and conditions, and its label says its state, so color is never the only cue.
 */
export function LevelPath({ level }: { level: number }) {
  return (
    <Tooltip.Provider delayDuration={300}>
      <ol aria-label="Level path" className="flex gap-11">
        {LEVEL_CONDITIONS.map((codes, i) => {
          const n = i + 1;
          const reached = n <= level;
          const state =
            n === level ? "your level" : reached ? "reached" : "not reached";
          return (
            <li
              key={n}
              className="relative grid w-5 justify-items-center gap-1.5"
            >
              {n > 1 && (
                <span
                  aria-hidden
                  className={cn(
                    "absolute top-[9px] right-full h-0.5 w-11",
                    reached ? "bg-primary" : "bg-wash",
                  )}
                />
              )}
              <Tooltip.Root>
                <Tooltip.Trigger asChild>
                  <span
                    tabIndex={0}
                    aria-label={`${levelTitle(n)}, ${state}`}
                    className={cn(
                      "grid size-5 place-items-center rounded-full outline-none focus-visible:ring-3 focus-visible:ring-ring/40 [&_svg]:size-3",
                      reached
                        ? "bg-primary text-primary-foreground"
                        : "bg-wash text-muted-foreground shadow-[inset_0_0_0_1px_var(--hairline)]",
                      n === level && "ring-3 ring-primary/25",
                    )}
                  >
                    {reached ? <Check aria-hidden /> : <Lock aria-hidden />}
                  </span>
                </Tooltip.Trigger>
                <Tooltip.Portal>
                  <Tooltip.Content
                    sideOffset={6}
                    className="glass-raised z-50 grid max-w-72 gap-1 rounded-toolbar px-3 py-2 text-small"
                  >
                    <b className="font-semibold">{levelTitle(n)}</b>
                    {codes.map((code) => (
                      <span key={code} className="text-muted-foreground">
                        {CONDITIONS[code]}
                      </span>
                    ))}
                  </Tooltip.Content>
                </Tooltip.Portal>
              </Tooltip.Root>
              <span
                aria-hidden
                className={cn(
                  "text-small tabular-nums",
                  n === level
                    ? "font-semibold text-foreground"
                    : "text-muted-foreground",
                )}
              >
                {n}
              </span>
            </li>
          );
        })}
      </ol>
    </Tooltip.Provider>
  );
}
