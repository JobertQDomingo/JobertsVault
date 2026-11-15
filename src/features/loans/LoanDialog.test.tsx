import { invoke } from "@tauri-apps/api/core";
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";
import type { NewLoan } from "@/bindings/NewLoan";
import {
  ACCOUNT_LIST,
  CATEGORIES,
  DEFAULTS,
  LOAN_CATEGORIES,
  LOANS,
} from "@/test/fixtures";
import { renderRoute, type Answers } from "@/test/render";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

const ANSWERS: Answers = {
  list_loans: LOANS,
  list_accounts: ACCOUNT_LIST,
  list_categories: LOAN_CATEGORIES,
  get_transaction_defaults: DEFAULTS,
  create_loan: ({ loan }: { loan: NewLoan }) => ({
    ...LOANS.loans[1],
    ...loan,
  }),
  update_loan: () => LOANS.loans[0],
};

const calls = (command: string) =>
  vi
    .mocked(invoke)
    .mock.calls.filter(([c]) => c === command)
    .map(([, args]) => args);
const dialog = () => within(screen.getByRole("dialog"));
const value = (name: string, role = "textbox") =>
  (dialog().getByRole(role, { name }) as HTMLInputElement).value;
const type = (name: string, text: string, role = "textbox") =>
  fireEvent.change(dialog().getByRole(role, { name }), {
    target: { value: text },
  });
const validation = (field: string, message: string) => () => {
  throw { code: "validation", field, message };
};

async function openAdd(answers: Answers = ANSWERS) {
  renderRoute("/debt-free", answers);
  fireEvent.click(await screen.findByRole("button", { name: "Add loan" }));
  await screen.findByRole("dialog", { name: "Add loan" });
}

async function openEdit(name: string) {
  renderRoute("/debt-free", ANSWERS);
  fireEvent.keyDown(
    await screen.findByRole("button", { name: `More actions for ${name}` }),
    { key: "Enter" },
  );
  fireEvent.click(screen.getByRole("menuitem", { name: "Edit" }));
  await screen.findByRole("dialog", { name: "Edit loan" });
}

test("a new loan starts as A new loan, in the Loan category, borrowed today, with one empty row (F12)", async () => {
  await openAdd();
  expect(value("Category", "combobox")).toBe("Loan");
  expect(
    dialog()
      .getByRole("radio", { name: "A new loan" })
      .getAttribute("aria-checked"),
  ).toBe("true");
  expect(value("Date borrowed")).toBe("11/05/2026");
  expect(value("Payment 1 date")).toBe("");
  expect(value("Payment 1 amount")).toBe("");
  dialog().getByText(
    "Optional. Add it to see what the loan costs in interest.",
  );
});

test("without an active category named Loan, the category starts empty", async () => {
  const archived = {
    ...LOAN_CATEGORIES,
    categories: LOAN_CATEGORIES.categories.map((c) =>
      c.name === "Loan" ? { ...c, archived: true } : c,
    ),
  };
  await openAdd({ ...ANSWERS, list_categories: archived });
  expect(value("Category", "combobox")).toBe("");
});

test("with no Loan category at all, the category starts empty", async () => {
  await openAdd({ ...ANSWERS, list_categories: CATEGORIES });
  expect(value("Category", "combobox")).toBe("");
});

test("a new row takes the previous amount, dated the first payment + n months, so month ends don't drift (Jan 31 → Feb 28 → Mar 31)", async () => {
  await openAdd();
  type("Payment 1 date", "01/31/2027");
  fireEvent.blur(dialog().getByRole("textbox", { name: "Payment 1 date" }));
  type("Payment 1 amount", "1000");
  fireEvent.click(dialog().getByRole("button", { name: "Add payment" }));
  await waitFor(() => expect(value("Payment 2 date")).toBe("02/28/2027"));
  expect(value("Payment 2 amount")).toBe("1,000.00");
  // Enter in the last amount adds a row, and doesn't save.
  const second = dialog().getByRole("textbox", { name: "Payment 2 amount" });
  fireEvent.keyDown(second, { key: "Enter" });
  await waitFor(() => expect(value("Payment 3 date")).toBe("03/31/2027"));
  expect(value("Payment 3 amount")).toBe("1,000.00");
  expect(document.activeElement).toBe(
    dialog().getByRole("textbox", { name: "Payment 3 amount" }),
  );
  expect(calls("create_loan")).toEqual([]);
});

test("a loan I already have marks rows before today Paid before, and doesn't start with today as the date borrowed", async () => {
  await openAdd();
  type("Payment 1 date", "10/28/2026");
  fireEvent.blur(dialog().getByRole("textbox", { name: "Payment 1 date" }));
  fireEvent.click(dialog().getByRole("button", { name: "Add payment" }));
  // A new loan has no paid-before rows.
  expect(dialog().queryByText("Paid before")).toBeNull();
  fireEvent.click(
    dialog().getByRole("radio", { name: "A loan I already have" }),
  );
  // Today as the date borrowed would restart the streak (DATA_MODEL §5.11).
  await waitFor(() => expect(value("Date borrowed")).toBe(""));
  const rows = within(
    dialog().getByRole("list", { name: "Payments" }),
  ).getAllByRole("listitem");
  within(rows[0]).getByText("Paid before");
  // Nov 28 is after today.
  expect(within(rows[1]).queryByText("Paid before")).toBeNull();
  fireEvent.click(dialog().getByRole("radio", { name: "A new loan" }));
  await waitFor(() => expect(value("Date borrowed")).toBe("11/05/2026"));
});

test("Save sends the loan and its rows, empty optional fields as none, and says Loan added", async () => {
  await openAdd();
  type("Name", "E-wallet loan");
  fireEvent.click(
    dialog().getByRole("radio", { name: "A loan I already have" }),
  );
  type("Payment 1 date", "12/15/2026");
  fireEvent.blur(dialog().getByRole("textbox", { name: "Payment 1 date" }));
  type("Payment 1 amount", "1000");
  fireEvent.click(dialog().getByRole("button", { name: "Add loan" }));
  await waitFor(() =>
    expect(calls("create_loan")).toEqual([
      {
        loan: {
          isNew: false,
          name: "E-wallet loan",
          // No account chosen goes as 0, for the backend to name.
          accountId: 0,
          categoryId: 6,
          amountBorrowed: null,
          borrowedOn: null,
          payments: [{ id: null, dueDate: "2026-12-15", amount: 100000 }],
        },
      },
    ]),
  );
  await screen.findByText("Loan added");
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
});

test.each([
  ["name", "Name", "A loan with this name already exists"],
  ["accountId", "Paid from", "This account is archived"],
  ["borrowedOn", "Date borrowed", "Enter the date borrowed"],
  ["amountBorrowed", "Amount borrowed", "Enter an amount above 0"],
])(
  "the backend's %s error shows under its field, and the dialog stays open",
  async (field, label, message) => {
    await openAdd({ ...ANSWERS, create_loan: validation(field, message) });
    fireEvent.click(dialog().getByRole("button", { name: "Add loan" }));
    const input = await waitFor(() => {
      const el = dialog().getByLabelText(label);
      expect(el.getAttribute("aria-invalid")).toBe("true");
      return el;
    });
    expect(
      document.getElementById(
        input.getAttribute("aria-describedby")!.split(" ").pop()!,
      )?.textContent,
    ).toBe(message);
  },
);

test("a row's error shows under the list and marks the row it names", async () => {
  await openAdd({
    ...ANSWERS,
    create_loan: validation("payments", "Payment 2: enter an amount above 0"),
  });
  fireEvent.click(dialog().getByRole("button", { name: "Add payment" }));
  fireEvent.click(dialog().getByRole("button", { name: "Add loan" }));
  await dialog().findByText("Payment 2: enter an amount above 0");
  expect(
    dialog()
      .getByRole("textbox", { name: "Payment 2 amount" })
      .getAttribute("aria-invalid"),
  ).toBe("true");
  expect(
    dialog()
      .getByRole("textbox", { name: "Payment 1 amount" })
      .getAttribute("aria-invalid"),
  ).toBeNull();
});

test("editing: paid rows are locked and left out, the rest keep their ids, and This is can't change (F12 step 3)", async () => {
  await openEdit("E-wallet loan");
  expect(dialog().queryByRole("radiogroup", { name: "This is" })).toBeNull();
  const rows = within(
    dialog().getByRole("list", { name: "Payments" }),
  ).getAllByRole("listitem");
  expect(rows).toHaveLength(8);
  // The paid row: text only, no inputs and no ✕.
  within(rows[2]).getByText("Paid Oct 27");
  within(rows[2]).getByText("10/28/2026");
  expect(within(rows[2]).queryByRole("textbox")).toBeNull();
  expect(within(rows[2]).queryByRole("button")).toBeNull();
  // Paid before goes by the day the loan was added (Oct 1), not today.
  within(rows[0]).getByText("Paid before");
  expect(within(rows[3]).queryByText("Paid before")).toBeNull();

  // A new row counts from the first payment, the paid one included: Aug 28 + 8 months.
  fireEvent.click(dialog().getByRole("button", { name: "Add payment" }));
  await waitFor(() => expect(value("Payment 9 date")).toBe("04/28/2027"));
  fireEvent.click(dialog().getByRole("button", { name: "Save" }));
  await waitFor(() => expect(calls("update_loan")).toHaveLength(1));
  const sent = calls("update_loan")[0] as {
    id: number;
    loan: { payments: { id: number | null; dueDate: string }[] };
  };
  expect(sent.id).toBe(1);
  expect(sent.loan.payments.map((p) => p.id)).toEqual([
    11,
    12,
    14,
    15,
    16,
    17,
    18,
    null,
  ]);
  await screen.findByText("Saved");
});

test("editing: a row's error names it among the rows sent, so it marks the right one past a paid row", async () => {
  renderRoute("/debt-free", {
    ...ANSWERS,
    update_loan: validation("payments", "Payment 3: enter a real date"),
  });
  fireEvent.keyDown(
    await screen.findByRole("button", {
      name: "More actions for E-wallet loan",
    }),
    { key: "Enter" },
  );
  fireEvent.click(screen.getByRole("menuitem", { name: "Edit" }));
  await screen.findByRole("dialog", { name: "Edit loan" });
  fireEvent.click(dialog().getByRole("button", { name: "Save" }));
  await dialog().findByText("Payment 3: enter a real date");
  // Sent row 3 is the 4th on screen: the paid Oct 28 row isn't sent.
  expect(
    dialog()
      .getByRole("textbox", { name: "Payment 4 date" })
      .getAttribute("aria-invalid"),
  ).toBe("true");
});
