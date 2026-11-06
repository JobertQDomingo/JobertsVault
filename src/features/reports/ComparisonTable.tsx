import type { ReactNode } from "react";
import type { Category } from "@/bindings/Category";
import type { CategoryLine } from "@/bindings/CategoryLine";
import type { Comparison } from "@/bindings/Comparison";
import type { Line } from "@/bindings/Line";
import { CategoryChip } from "@/components/CategoryChip";
import { Change } from "@/components/Change";
import { MoneyText } from "@/components/MoneyText";
import { formatSpan } from "@/lib/dates";
import { cn } from "@/lib/utils";

type Kind = "income" | "expense" | "net";

function Row({
  name,
  line,
  kind,
  total = false,
}: {
  name: ReactNode;
  line: Line;
  kind: Kind;
  total?: boolean;
}) {
  const cell = "px-3 py-2";
  // The group says income or expense, so amounts are plain and the difference adds up as shown;
  // only net cashflow, which can be either, keeps its sign.
  const shown = kind === "net" ? "net" : "balance";
  return (
    <tr className={cn("border-t border-hairline", total && "bg-wash")}>
      <th
        scope="row"
        className={cn(
          "py-2 pr-3 text-left",
          total ? "pl-3 font-semibold" : "pl-6 font-normal",
        )}
      >
        {name}
      </th>
      <td className={cell}>
        <MoneyText amount={line.current} kind={shown} />
      </td>
      <td className={cell}>
        <MoneyText amount={line.compare} kind={shown} />
      </td>
      <td className={cell}>
        <MoneyText amount={line.difference} kind="difference" />
      </td>
      <td className={cell}>
        <Change tenths={line.change} upIsGood={kind !== "expense"} />
      </td>
    </tr>
  );
}

/**
 * The period comparison (RPT-4, UI_FLOWS §8.1): income, expense, each category and net cashflow,
 * this period next to the dates it's compared with (RPT-4a), with the difference and % change. Every
 * figure is the backend's.
 */
export function ComparisonTable({
  comparison: c,
  categories,
}: {
  comparison: Comparison;
  categories: Map<number, Category>;
}) {
  const current = formatSpan(c.currentStart, c.currentEnd, c.today);
  const compare = formatSpan(c.compareStart, c.compareEnd, c.today);
  const rows = (lines: CategoryLine[], kind: Kind) =>
    lines.map((l) => {
      const category = categories.get(l.categoryId);
      return (
        <Row
          key={l.categoryId}
          kind={kind}
          line={l}
          name={
            category ? (
              <CategoryChip
                color={category.color}
                icon={category.icon}
                name={category.name}
                size="sm"
              />
            ) : (
              "…"
            )
          }
        />
      );
    });
  return (
    <table className="w-full text-right">
      <caption className="sr-only">
        {current} compared with {compare}
      </caption>
      <colgroup>
        <col />
        <col className="w-40" />
        <col className="w-40" />
        <col className="w-40" />
        <col className="w-32" />
      </colgroup>
      <thead className="text-small text-muted-foreground">
        <tr>
          <th scope="col" className="py-2 pl-3 text-left font-semibold">
            <span className="sr-only">Figure</span>
          </th>
          {[current, compare, "Difference", "Change"].map((label) => (
            <th key={label} scope="col" className="px-3 py-2 font-semibold">
              {label}
            </th>
          ))}
        </tr>
      </thead>
      <tbody>
        <Row name="Income" line={c.income} kind="income" total />
        {rows(c.incomeCategories, "income")}
        <Row name="Expense" line={c.expense} kind="expense" total />
        {rows(c.expenseCategories, "expense")}
      </tbody>
      <tfoot>
        <Row name="Net cashflow" line={c.net} kind="net" total />
      </tfoot>
    </table>
  );
}
