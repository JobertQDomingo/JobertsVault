import { fireEvent, screen, within } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import type { Account } from "@/bindings/Account";
import type { Category } from "@/bindings/Category";
import type { Transaction } from "@/bindings/Transaction";
import { renderWithSettings } from "@/test/render";
import { AccountRow } from "./AccountRow";
import { CategoryChip } from "./CategoryChip";
import { TransactionRow } from "./TransactionRow";

const account = (over: Partial<Account>): Account => ({
  id: 1,
  name: "Cash",
  kind: "cash",
  openingBalance: 0,
  creditLimit: null,
  statementDay: null,
  dueDay: null,
  sortOrder: 0,
  archived: false,
  balance: 32000,
  card: null,
  inUse: true,
  ...over,
});
const CASH = account({});
const BANK = account({ id: 2, name: "Bank", kind: "bank", balance: 841075 });
const CARD = account({
  id: 4,
  name: "Credit card",
  kind: "credit_card",
  creditLimit: 1000000,
  balance: -64000,
  card: {
    amountOwed: 64000,
    availableCredit: 936000,
    utilizationPercent: 6,
    cycleStart: "2026-10-21",
    cycleEnd: "2026-11-20",
    cycleSpending: 41230,
    dueDate: "2026-11-05",
  },
});
const GROCERIES: Category = {
  id: 9,
  name: "Food & Groceries",
  kind: "expense",
  color: "cat-1",
  icon: "shopping-cart",
  sortOrder: 0,
  archived: false,
  transactionCount: 3,
  inUse: true,
};
const txn = (over: Partial<Transaction>): Transaction => ({
  id: 7,
  kind: "expense",
  amount: 8520,
  date: "2026-10-26",
  accountId: 1,
  toAccountId: null,
  categoryId: 9,
  payee: "Market",
  paymentMethodId: null,
  note: "weekly shop",
  ...over,
});

test("a chip tints the category color and shows its icon", () => {
  const { container } = renderWithSettings(
    <CategoryChip color="cat-3" icon="utensils" />,
  );
  const chip = container.firstElementChild as HTMLElement;
  expect(chip.style.getPropertyValue("--chip")).toBe("var(--cat-3)");
  expect(chip.querySelector("svg.lucide-utensils")).toBeTruthy();
});

test("a chip with an unknown icon still draws one", () => {
  const { container } = renderWithSettings(
    <CategoryChip color="cat-3" icon="no-such-icon" size="sm" />,
  );
  expect(container.querySelector("svg")).toBeTruthy();
  expect((container.firstElementChild as HTMLElement).className).toContain(
    "size-6",
  );
});

test("an account row shows its icon, name and balance", () => {
  renderWithSettings(<AccountRow account={BANK} />);
  expect(screen.getByText("Bank")).toBeTruthy();
  expect(screen.getByText("₱8,410.75")).toBeTruthy();
  expect(document.querySelector("svg.lucide-landmark")).toBeTruthy();
});

test("a negative balance is red with a minus", () => {
  renderWithSettings(<AccountRow account={account({ balance: -4500 })} />);
  expect(screen.getByText("−₱45.00").className).toContain("text-expense");
});

test("a card shows what's owed, what's available and when it's due", () => {
  renderWithSettings(<AccountRow account={CARD} />);
  expect(screen.getByText("₱640.00 owed")).toBeTruthy();
  expect(
    screen.getByText(
      (_, el) =>
        el?.textContent === "Available ₱9,360.00 · Due Nov 5" &&
        el.tagName === "P",
    ),
  ).toBeTruthy();
});

test("an expense row shows the category, payee and note, account and amount", () => {
  renderWithSettings(
    <TransactionRow
      transaction={txn({})}
      category={GROCERIES}
      account={CASH}
    />,
  );
  expect(screen.getByText("Food & Groceries")).toBeTruthy();
  expect(screen.getByText("Market · weekly shop")).toBeTruthy();
  expect(screen.getByText("Cash")).toBeTruthy();
  expect(screen.getByText("−₱85.20")).toBeTruthy();
});

test("a transfer row shows both accounts and no sign", () => {
  renderWithSettings(
    <TransactionRow
      transaction={txn({
        kind: "transfer",
        amount: 120000,
        accountId: 2,
        toAccountId: 4,
        categoryId: null,
        payee: null,
        note: "October bill",
      })}
      account={BANK}
      toAccount={CARD}
    />,
  );
  expect(screen.getByText("Transfer")).toBeTruthy();
  expect(screen.getByText("Bank → Credit card")).toBeTruthy();
  expect(screen.getByText("₱1,200.00")).toBeTruthy();
});

test("a row whose category isn't in the list still shows", () => {
  renderWithSettings(<TransactionRow transaction={txn({})} account={CASH} />);
  expect(screen.getByText("−₱85.20")).toBeTruthy();
});

test("clicking a row or ✎ opens it, ⧉ duplicates it", () => {
  const onOpen = vi.fn();
  const onDuplicate = vi.fn();
  renderWithSettings(
    <TransactionRow
      transaction={txn({})}
      category={GROCERIES}
      account={CASH}
      onOpen={onOpen}
      onDuplicate={onDuplicate}
    />,
  );
  fireEvent.click(screen.getByText("Market · weekly shop"));
  fireEvent.click(screen.getByRole("button", { name: "Edit" }));
  expect(onOpen).toHaveBeenCalledTimes(2);
  fireEvent.click(screen.getByRole("button", { name: "Duplicate" }));
  expect(onDuplicate).toHaveBeenCalledOnce();
  expect(onOpen).toHaveBeenCalledTimes(2);
});

test("a compact row (dashboard) has no account column or buttons", () => {
  const { container } = renderWithSettings(
    <TransactionRow
      compact
      transaction={txn({})}
      category={GROCERIES}
      account={CASH}
      onOpen={() => {}}
    />,
  );
  const row = within(container.firstElementChild as HTMLElement);
  expect(row.queryByText("Cash")).toBeNull();
  expect(row.queryByRole("button", { name: "Duplicate" })).toBeNull();
  expect(row.getByText("Food & Groceries · Market")).toBeTruthy();
});

test("a card past its limit shows negative available credit in red", () => {
  renderWithSettings(
    <AccountRow
      account={{ ...CARD, card: { ...CARD.card!, availableCredit: -200000 } }}
    />,
  );
  expect(screen.getByText("−₱2,000.00").className).toContain("text-expense");
});

test("an overpaid card shows its credit, not ₱0.00 owed", () => {
  renderWithSettings(
    <AccountRow
      account={{
        ...CARD,
        balance: 50000,
        card: { ...CARD.card!, amountOwed: 0 },
      }}
    />,
  );
  expect(screen.getByText("₱500.00")).toBeTruthy();
  expect(screen.queryByText(/owed/)).toBeNull();
});

test("a row only offers the actions it was given", () => {
  renderWithSettings(
    <TransactionRow
      transaction={txn({})}
      category={GROCERIES}
      account={CASH}
      onOpen={() => {}}
    />,
  );
  expect(screen.getByRole("button", { name: "Edit" })).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Duplicate" })).toBeNull();
});

test("a compact row's button holds only inline content", () => {
  renderWithSettings(
    <TransactionRow
      compact
      transaction={txn({})}
      category={GROCERIES}
      account={CASH}
      onOpen={() => {}}
    />,
  );
  expect(screen.getByRole("button").querySelector("p, div")).toBeNull();
});
