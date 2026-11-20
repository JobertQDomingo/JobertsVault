import {
  PolarAngleAxis,
  PolarGrid,
  PolarRadiusAxis,
  Radar,
  RadarChart,
} from "recharts";
import type { Snapshot } from "@/bindings/Snapshot";
import type { StatName } from "@/bindings/StatName";
import { cn } from "@/lib/utils";
import { STAT_LABELS } from "@/lib/vault";

/**
 * The six stats as a radar chart (DESIGN_SYSTEM §9.2), 280 px and flat inside its panel: today's
 * shape over last check-in's dashed outline, a corner only for each stat that applies, and no
 * chart with fewer than 3 of them. It's hidden from screen readers, which get the table instead.
 */
export function StatHexagon({
  stats,
  previous,
}: {
  /** In DATA_MODEL §5.20's order; a null score has no corner. */
  stats: { stat: StatName; score: number | null }[];
  /** Last check-in's scores, in the same order, or none before the first one. */
  previous?: (number | null)[] | null;
}) {
  const data = stats.flatMap(({ stat, score }, i) =>
    score === null
      ? []
      : [{ name: STAT_LABELS[stat], score, prev: previous?.[i] ?? null }],
  );
  // The outline only when every corner had a score then, so it never folds into the middle.
  const outline = !!previous && data.every((d) => d.prev !== null);
  return (
    <>
      {data.length >= 3 && (
        <div aria-hidden className="size-[280px] shrink-0">
          <RadarChart width={280} height={280} data={data} outerRadius={96}>
            <PolarGrid stroke="var(--wash)" />
            <PolarRadiusAxis
              domain={[0, 100]}
              tickCount={5}
              tick={false}
              axisLine={false}
            />
            <PolarAngleAxis
              dataKey="name"
              tickLine={false}
              tick={{ fill: "var(--muted-foreground)", fontSize: 12 }}
            />
            {outline && (
              <Radar
                dataKey="prev"
                stroke="var(--muted-foreground)"
                strokeWidth={1.5}
                strokeDasharray="4 3"
                fill="none"
                isAnimationActive={false}
              />
            )}
            <Radar
              dataKey="score"
              stroke="var(--primary)"
              strokeWidth={2}
              fill="var(--primary)"
              fillOpacity={0.24}
              isAnimationActive={false}
            />
          </RadarChart>
        </div>
      )}
      <table className="sr-only">
        <caption>Stats</caption>
        <thead>
          <tr>
            <th scope="col">Stat</th>
            <th scope="col">Score</th>
            {previous && <th scope="col">Last check-in</th>}
          </tr>
        </thead>
        <tbody>
          {stats.map(({ stat, score }, i) => (
            <tr key={stat}>
              <th scope="row">{STAT_LABELS[stat]}</th>
              <td>{score ?? "—"}</td>
              {previous && <td>{previous[i] ?? "—"}</td>}
            </tr>
          ))}
        </tbody>
      </table>
    </>
  );
}

/**
 * The six scores beside the chart: the name, the score ("—" when it doesn't apply) and, from the
 * first check-in on, `▲ 6`, `▼ 10` or `—` against it (UI_FLOWS §8.7–8.8). Given `history`, from 3
 * check-ins on each row ends with its trend line (LVL-8).
 */
export function StatScores({
  stats,
  previous,
  history,
}: {
  stats: { stat: StatName; score: number | null }[];
  previous?: (number | null)[] | null;
  /** The latest snapshots, oldest first. */
  history?: Snapshot[];
}) {
  const trends = !!history && history.length >= 3;
  return stats.map(({ stat, score }, i) => {
    const was = previous?.[i] ?? null;
    return (
      <div
        key={stat}
        className={cn(
          "grid h-8 items-center gap-2",
          trends
            ? "grid-cols-[76px_36px_48px_64px]"
            : "grid-cols-[76px_36px_48px]",
        )}
      >
        <span>{STAT_LABELS[stat]}</span>
        <b className="text-right font-semibold tabular-nums">{score ?? "—"}</b>
        {previous &&
          (score === null ? (
            <span className="text-muted-foreground">—</span>
          ) : (
            <Points change={was === null ? null : score - was} />
          ))}
        {trends && <Trend points={history.map((h) => h.stats[i] ?? null)} />}
      </div>
    );
  });
}

/**
 * A stat's snapshots as a 64 × 20 px line with no axes or dots (UI_FLOWS §8.8), scaled to its own
 * range but never less than 20 points tall, so a flat line sits in the middle. A snapshot where the
 * stat didn't apply breaks the line, so it never suggests a value there. Hidden from screen readers:
 * the score and its change say it.
 */
function Trend({ points }: { points: (number | null)[] }) {
  const known = points.filter((p) => p !== null);
  const span = Math.max(Math.max(...known) - Math.min(...known), 20);
  const bottom = (Math.max(...known) + Math.min(...known) - span) / 2;
  const step = 62 / Math.max(points.length - 1, 1);
  // Runs of points between the gaps; 1 px inside each edge, so the 1.5 px stroke isn't clipped.
  const runs: string[][] = [[]];
  points.forEach((p, i) => {
    if (p === null) runs.push([]);
    else
      runs[runs.length - 1].push(
        `${1 + i * step},${19 - ((p - bottom) / span) * 18}`,
      );
  });
  return (
    <svg aria-hidden data-trend width={64} height={20}>
      {runs
        .filter((run) => run.length >= 2)
        .map((run) => (
          <polyline
            key={run[0]}
            points={run.join(" ")}
            fill="none"
            stroke="var(--primary)"
            strokeWidth={1.5}
            strokeLinejoin="round"
            strokeLinecap="round"
          />
        ))}
    </svg>
  );
}

/** `▲ 6` or `▼ 10`, the arrow spelled out for screen readers; `—` when it's the same or unknown. */
function Points({ change }: { change: number | null }) {
  if (!change)
    return (
      <span className="text-muted-foreground">
        —
        <span className="sr-only">
          {change === 0 ? " No change" : " No earlier score"}
        </span>
      </span>
    );
  const up = change > 0;
  return (
    <span
      className={cn(
        "font-semibold tabular-nums",
        up ? "text-income" : "text-expense",
      )}
    >
      <span aria-hidden>{up ? "▲ " : "▼ "}</span>
      <span className="sr-only">{up ? "Up " : "Down "}</span>
      {Math.abs(change)}
    </span>
  );
}
