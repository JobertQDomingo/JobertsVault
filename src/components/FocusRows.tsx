import { Target } from "lucide-react";
import type { ActiveFocus } from "@/bindings/ActiveFocus";
import { ProgressBar } from "@/components/ProgressBar";
import { formatShortDate } from "@/lib/dates";
import { type FocusNames, focusText, focusUnit } from "@/lib/vault";

const days = (n: number) => (n === 1 ? "1 day" : `${n} days`);

/** A focus's progress so far, after "·" (UI_FLOWS §2's strip). */
function progress(f: ActiveFocus, money: FocusNames["money"]) {
  const actual = f.actual ?? 0;
  switch (f.kind) {
    case "category_cap":
    case "spending_cap":
      return `${money(actual)} so far`;
    case "card_level":
      return `${money(actual)} this cycle`;
    case "loan_on_time":
      return f.paidOn ? `paid ${formatShortDate(f.paidOn)}` : "not paid yet";
    case "logging":
      return `${days(actual)} so far`;
  }
}

/**
 * The active focuses, the month's first, as flat rows (UI_FLOWS §8.8, DESIGN_SYSTEM §9.2's
 * `FocusStrip`): the sentence, its progress, and for a cap a `ProgressBar` under it.
 */
export function FocusRows({
  focuses,
  names,
  level,
}: {
  focuses: ActiveFocus[];
  names: FocusNames;
  /** A card-level focus's level name ("mid"), when its target is one of the card's levels. */
  level: (f: ActiveFocus) => string | null;
}) {
  const sorted = [...focuses].sort(
    (a, b) => +(focusUnit(b) === "month") - +(focusUnit(a) === "month"),
  );
  return (
    <ul className="grid divide-y divide-hairline">
      {sorted.map((f) => {
        // The card's sentence ends "this cycle", which its progress already says.
        const text = focusText(f, names, level(f)).replace(/ this cycle$/, "");
        return (
          <li
            key={`${f.kind}-${f.start}-${f.end}`}
            className="grid gap-2 py-2.5 first:pt-0 last:pb-0"
          >
            <p className="flex items-start gap-2 tabular-nums">
              <Target
                aria-hidden
                className="mt-1 size-4 shrink-0 text-primary"
              />
              <span>
                {`This ${focusUnit(f)}'s focus: ${text} `}
                <span className="text-muted-foreground">
                  {`· ${progress(f, names.money)}`}
                </span>
              </span>
            </p>
            {f.percent !== null && (
              <ProgressBar
                percent={f.percent}
                state={f.state}
                aria-label={`${text}: ${f.percent}%`}
                className="pl-6"
              />
            )}
          </li>
        );
      })}
    </ul>
  );
}
