import { invoke } from "@tauri-apps/api/core";
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";
import type { Payslip } from "@/bindings/Payslip";
import type { PayslipFields } from "@/bindings/PayslipFields";
import {
  ACCOUNT_LIST,
  CATEGORIES,
  LIST,
  PAYMENT_METHODS,
  PERIOD,
} from "@/test/fixtures";
import { renderRoute, type Answers } from "@/test/render";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

const expense = (id: number, name: string) => ({
  ...CATEGORIES.categories[0],
  id,
  name,
  sortOrder: id,
});
// "Utilities" comes before the deductions in the category order; the form lists them first.
const WITH_DEDUCTIONS = {
  ...CATEGORIES,
  categories: [
    ...CATEGORIES.categories,
    expense(6, "Utilities"),
    expense(7, "SSS"),
    expense(8, "PhilHealth"),
  ],
};

/** The Nov 4 salary row (id 4) is a payslip's gross row with two deductions. */
const PAYSLIP: Payslip = {
  id: 4,
  categoryId: 3,
  date: "2026-11-04",
  accountId: 2,
  gross: 460000,
  deductions: [
    { categoryId: 7, amount: 18000 },
    { categoryId: 8, amount: 11500 },
  ],
  note: null,
};
const net = (p: PayslipFields) =>
  p.gross - p.deductions.reduce((sum, d) => sum + d.amount, 0);

function renderPayslip(answers: Answers = {}) {
  renderRoute("/transactions", {
    get_period: PERIOD,
    list_transactions: {
      ...LIST,
      items: LIST.items.map((t) => (t.id === 4 ? { ...t, payslipId: 4 } : t)),
    },
    list_accounts: ACCOUNT_LIST,
    list_categories: WITH_DEDUCTIONS,
    list_payment_methods: PAYMENT_METHODS,
    get_payslip: PAYSLIP,
    // Stands in for the backend's answer; the form only shows it.
    preview_payslip: ({ payslip }: { payslip: PayslipFields }) => ({
      net: net(payslip),
    }),
    update_payslip: PAYSLIP,
    delete_transaction: null,
    ...answers,
  });
}

const calls = (command: string) =>
  vi
    .mocked(invoke)
    .mock.calls.filter(([c]) => c === command)
    .map(([, args]) => args);

async function openPayslip(answers?: Answers) {
  renderPayslip(answers);
  fireEvent.click(await screen.findByText("Employer"));
  const form = within(
    await screen.findByRole("dialog", { name: "Edit payslip" }),
  );
  // The form shows once its payslip has loaded.
  await form.findByRole("textbox", { name: "Gross pay" });
  return form;
}

test("a payslip row opens its payslip, with the net from the backend (F14)", async () => {
  const form = await openPayslip();
  expect(calls("get_payslip")).toEqual([{ id: 4 }]);
  expect(await form.findByText("₱4,305.00")).toBeTruthy();
  const category = (n: number) =>
    (
      form.getByRole("combobox", {
        name: `Deduction ${n} category`,
      }) as HTMLInputElement
    ).value;
  expect([category(1), category(2)]).toEqual(["SSS", "PhilHealth"]);
  expect(screen.queryByRole("dialog", { name: "Edit transaction" })).toBeNull();
});

test("the deduction categories come first, then the other expense categories", async () => {
  const form = await openPayslip();
  fireEvent.click(form.getByRole("combobox", { name: "Deduction 1 category" }));
  const options = screen.getAllByRole("option").map((o) => o.textContent);
  expect(options).toEqual([
    "SSS",
    "PhilHealth",
    "Food & Groceries",
    "Dining Out",
    "Transportation",
    "Utilities",
  ]);
});

test("the net follows the lines as they change, and is flagged when it isn't above 0", async () => {
  const form = await openPayslip();
  await form.findByText("₱4,305.00");
  fireEvent.click(form.getByRole("button", { name: "Remove deduction 2" }));
  expect(await form.findByText("₱4,420.00")).toBeTruthy();

  fireEvent.click(form.getByRole("button", { name: "Add deduction" }));
  fireEvent.change(form.getByRole("textbox", { name: "Deduction 2 amount" }), {
    target: { value: "5000" },
  });
  expect(
    await form.findByText("Deductions can't add up to the gross pay or more"),
  ).toBeTruthy();
  expect(form.getByTestId("payslip-net").className).toContain("text-danger");
});

test("saving an edit sends the whole payslip, and a line's error shows under it", async () => {
  const form = await openPayslip({
    update_payslip: () => {
      throw {
        code: "validation",
        field: "deductions.1.amount",
        message: "Enter an amount above 0",
      };
    },
  });
  await form.findByText("₱4,305.00");
  fireEvent.click(form.getByRole("button", { name: "Save" }));
  expect(await form.findByText("Enter an amount above 0")).toBeTruthy();
  expect(calls("update_payslip")).toEqual([
    {
      id: 4,
      payslip: {
        date: "2026-11-04",
        accountId: 2,
        gross: 460000,
        deductions: PAYSLIP.deductions,
        note: null,
      },
    },
  ]);
});

test("Delete removes the whole payslip after saying what goes", async () => {
  const form = await openPayslip();
  fireEvent.click(form.getByRole("button", { name: "Delete" }));
  expect(
    await screen.findByText(
      "Its gross pay and 2 deductions are deleted. Its payday won't be offered again, so to fix a mistake, edit the payslip instead.",
    ),
  ).toBeTruthy();
  const confirm = within(
    screen.getByRole("dialog", { name: "Delete this payslip?" }),
  );
  fireEvent.click(confirm.getByRole("button", { name: "Delete" }));
  expect(await screen.findByText("Deleted")).toBeTruthy();
  expect(calls("delete_transaction")).toEqual([{ id: 4 }]);
  await waitFor(() =>
    expect(screen.queryByRole("dialog", { name: "Edit payslip" })).toBeNull(),
  );
});
