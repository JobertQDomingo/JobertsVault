import { Link } from "react-router";
import {
  Cell,
  Pie,
  PieChart,
  Tooltip,
  type TooltipContentProps,
} from "recharts";
import type {
  NameType,
  ValueType,
} from "recharts/types/component/DefaultTooltipContent";
import type { Breakdown } from "@/bindings/Breakdown";
import type { Category } from "@/bindings/Category";
import type { CategoryKind } from "@/bindings/CategoryKind";
import type { PeriodQuery } from "@/bindings/PeriodQuery";
import { CategoryChip } from "@/components/CategoryChip";
import { MoneyText } from "@/components/MoneyText";
import { writePeriod } from "@/hooks/usePeriodQuery";
import { formatShare } from "@/lib/money";

const SIZE = 216;

type Part = {
  key: string;
  name: string;
  fill: string;
  amount: number;
  share: number;
  kind: CategoryKind;
};

/** Flat and solid, like the chart's: it floats inside a blurred panel (DESIGN_SYSTEM §2.3). */
function SliceTooltip({
  active,
  payload,
}: TooltipContentProps<ValueType, NameType>) {
  const part = payload?.[0]?.payload as Part | undefined;
  if (!active || !part) return null;
  return (
    <div className="grid gap-0.5 rounded-control border border-edge bg-glass-solid px-3 py-2 text-small shadow-raised">
      <p className="font-semibold">{part.name}</p>
      <p className="text-muted-foreground">
        <MoneyText amount={part.amount} kind={part.kind} /> ·{" "}
        {formatShare(part.share)}
      </p>
    </div>
  );
}

/**
 * The category breakdown (RPT-2, UI_FLOWS §8.1): a donut of the 8 largest categories plus Other,
 * and every category ranked by amount with its share. Each row opens the transaction list for that
 * category and period (RPT-3). The slices and shares are the backend's.
 */
export function CategoryBreakdown({
  breakdown,
  categories,
  query,
  accountId,
}: {
  breakdown: Breakdown;
  categories: Map<number, Category>;
  query: PeriodQuery;
  /** The report's account filter, kept on the category's transactions (RPT-5). */
  accountId: number | null;
}) {
  // The data's own kind: while the other kind loads, these figures stay on screen.
  const { kind } = breakdown;
  const label = kind === "expense" ? "expenses" : "income";
  if (breakdown.categories.length === 0)
    return (
      <p className="py-10 text-center text-muted-foreground">
        No {label} recorded in this period.
      </p>
    );
  const parts: Part[] = breakdown.slices.map((s) => {
    const category =
      s.categoryId === null ? undefined : categories.get(s.categoryId);
    return {
      key: String(s.categoryId ?? "other"),
      // Only the rest is Other; a category the list doesn't have yet is unnamed, not Other.
      name: s.categoryId === null ? "Other" : (category?.name ?? "…"),
      fill: category ? `var(--${category.color})` : "var(--muted-foreground)",
      amount: s.amount,
      share: s.share,
      kind,
    };
  });
  const link = (categoryId: number) => {
    const params = writePeriod(new URLSearchParams(), query);
    params.set("category", String(categoryId));
    if (accountId !== null) params.set("account", String(accountId));
    return `/transactions?${params}`;
  };
  return (
    <div className="flex items-start gap-8">
      <div className="relative shrink-0" style={{ width: SIZE, height: SIZE }}>
        {/* The ranked list says everything the donut does, for screen readers too. */}
        <div aria-hidden>
          <PieChart width={SIZE} height={SIZE} accessibilityLayer={false}>
            <Pie
              data={parts}
              dataKey="amount"
              nameKey="name"
              innerRadius={SIZE / 2 - 36}
              outerRadius={SIZE / 2 - 2}
              startAngle={90}
              endAngle={-270}
              paddingAngle={parts.length > 1 ? 1.5 : 0}
              stroke="none"
              isAnimationActive={false}
            >
              {parts.map((p) => (
                <Cell key={p.key} fill={p.fill} />
              ))}
            </Pie>
            <Tooltip content={SliceTooltip} isAnimationActive={false} />
          </PieChart>
        </div>
        <div className="pointer-events-none absolute inset-0 grid place-content-center justify-items-center gap-0.5">
          <span className="text-small text-muted-foreground">
            Total {label}
          </span>
          <MoneyText amount={breakdown.total} kind={kind} />
        </div>
      </div>
      <ol aria-label={`Categories by ${label}`} className="grid min-w-0 flex-1">
        {breakdown.categories.map((c) => {
          const category = categories.get(c.categoryId);
          return (
            <li key={c.categoryId}>
              <Link
                to={link(c.categoryId)}
                title="Show these transactions"
                className="grid grid-cols-[minmax(0,1fr)_4rem_9rem] items-center gap-4 rounded-control px-3 py-1.5 transition-colors duration-150 hover:bg-glass-hover"
              >
                {category ? (
                  <CategoryChip
                    color={category.color}
                    icon={category.icon}
                    name={category.name}
                    size="sm"
                  />
                ) : (
                  <span />
                )}
                <span className="text-right text-muted-foreground tabular-nums">
                  {formatShare(c.share)}
                </span>
                <MoneyText
                  amount={c.amount}
                  kind={kind}
                  className="justify-self-end"
                />
              </Link>
            </li>
          );
        })}
      </ol>
    </div>
  );
}
