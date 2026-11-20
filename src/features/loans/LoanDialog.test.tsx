import { invoke } from "@tauri-apps/api/core";
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";
import type { NewLoan } from "@/bindings/NewLoan";
import {
  ACCOUNT_LIST,
  CATEGORIES,
  DEFAULTS,
  LOAN_CATEGORIES,
  LOAN_PREVIEW,
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
  preview_loan: LOAN_PREVIEW,
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

// A new loan's Save first shows "Before you add this loan" (LOAN-10).
const addAnyway = async () =>
  fireEvent.click(
    await screen.findByRole("button", { name: "Add this loan anyway" }),
  );

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

test("the total follows the rows as they're typed, added, edited and removed, from the backend (F12)", async () => {
  await openAdd();
  const total = () => dialog().getByTestId("loan-total");
  await waitFor(() => expect(total().textContent).toBe("Total ₱0.00"));
  type("Payment 1 amount", "1000");
  await waitFor(() => expect(total().textContent).toBe("Total ₱1,000.00"));
  fireEvent.click(dialog().getByRole("button", { name: "Add payment" }));
  await waitFor(() => expect(total().textContent).toBe("Total ₱2,000.00"));
  type("Payment 2 amount", "500");
  await waitFor(() => expect(total().textContent).toBe("Total ₱1,500.00"));
  fireEvent.click(dialog().getByRole("button", { name: "Remove payment 1" }));
  await waitFor(() => expect(total().textContent).toBe("Total ₱500.00"));
  // The frontend only sends the rows; the backend adds them up (CLAUDE.md rule 2).
  expect(calls("total_loan_payments").pop()).toEqual({
    rows: [{ amount: 50000, paid: false }],
  });
});

test("on a loan I already have, a paid-before row splits the total from what's still to pay", async () => {
  await openAdd();
  type("Payment 1 date", "10/28/2026");
  fireEvent.blur(dialog().getByRole("textbox", { name: "Payment 1 date" }));
  type("Payment 1 amount", "1000");
  fireEvent.click(dialog().getByRole("button", { name: "Add payment" }));
  fireEvent.click(
    dialog().getByRole("radio", { name: "A loan I already have" }),
  );
  await waitFor(() =>
    expect(dialog().getByTestId("loan-total").textContent).toBe(
      "Total ₱2,000.00 · Still to pay ₱1,000.00",
    ),
  );
  // A paid-before row with no amount yet still shows both figures (F12).
  type("Payment 1 amount", "");
  await waitFor(() =>
    expect(dialog().getByTestId("loan-total").textContent).toBe(
      "Total ₱1,000.00 · Still to pay ₱1,000.00",
    ),
  );
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
  // A loan the user already has skips "Before you add this loan" (F12 step 2).
  expect(calls("preview_loan")).toEqual([]);
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
    // The backend checks everything else only on the save after the confirm.
    await addAnyway();
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
  await addAnyway();
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

test("editing: the total counts the locked paid and paid-before rows, and still to pay doesn't (F12)", async () => {
  await openEdit("E-wallet loan");
  const total = () => dialog().getByTestId("loan-total").textContent;
  // 8 rows of ₱1,000.00: two paid before the app, one marked paid in it.
  await waitFor(() =>
    expect(total()).toBe("Total ₱8,000.00 · Still to pay ₱5,000.00"),
  );
  fireEvent.click(dialog().getByRole("button", { name: "Remove payment 3" }));
  await waitFor(() =>
    expect(total()).toBe("Total ₱7,000.00 · Still to pay ₱4,000.00"),
  );
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
  // It's the 8th editable row: the locked one has no number.
  await waitFor(() => expect(value("Payment 8 date")).toBe("04/28/2027"));
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
  // Sent row 3 is the 4th on screen, past the paid Oct 28 row, and its label says 3 too.
  expect(
    dialog()
      .getByRole("textbox", { name: "Payment 3 date" })
      .getAttribute("aria-invalid"),
  ).toBe("true");
});

test("a new loan first shows Before you add this loan from its typed rows, and Go back returns to the form (LOAN-10)", async () => {
  await openAdd();
  type("Name", "E-wallet loan");
  type("Payment 1 date", "12/15/2026");
  fireEvent.blur(dialog().getByRole("textbox", { name: "Payment 1 date" }));
  type("Payment 1 amount", "1000");
  fireEvent.click(dialog().getByRole("button", { name: "Add loan" }));
  const check = within(
    await screen.findByRole("dialog", { name: "Before you add this loan" }),
  );
  expect(calls("preview_loan")).toEqual([
    {
      draft: {
        payments: [{ id: null, dueDate: "2026-12-15", amount: 100000 }],
      },
    },
  ]);
  check.getByText(
    "Your debt-free date moves from Sun, Mar 28, 2027 to Wed, May 5, 2027.",
  );
  check.getByText("Safe to spend drops from ₱8,400.00 to ₱7,400.00.");
  const short = check.getByText(/would run short by ₱180\.00\./);
  expect(short.className).toContain("text-danger");
  // Only the window it makes short is named.
  expect(check.queryByText(/Dec 15–29/)).toBeNull();
  expect(
    check.getByRole("button", { name: "Add this loan anyway" }).className,
  ).toContain("bg-danger");

  fireEvent.click(check.getByRole("button", { name: "Go back" }));
  await waitFor(() =>
    expect(
      screen.queryByRole("dialog", { name: "Before you add this loan" }),
    ).toBeNull(),
  );
  screen.getByRole("dialog", { name: "Add loan" });
  expect(value("Name")).toBe("E-wallet loan");
  expect(calls("create_loan")).toEqual([]);
});

test("Add this loan anyway saves it as a new loan", async () => {
  await openAdd();
  type("Payment 1 date", "12/15/2026");
  fireEvent.blur(dialog().getByRole("textbox", { name: "Payment 1 date" }));
  type("Payment 1 amount", "1000");
  fireEvent.click(dialog().getByRole("button", { name: "Add loan" }));
  await addAnyway();
  await screen.findByText("Loan added");
  expect(calls("create_loan")).toMatchObject([
    {
      loan: {
        isNew: true,
        payments: [{ id: null, dueDate: "2026-12-15", amount: 100000 }],
      },
    },
  ]);
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
});

test("with nothing owed and no payday rule, the confirm says when you'd be debt-free and nothing about Safe to spend", async () => {
  await openAdd({
    ...ANSWERS,
    preview_loan: {
      ...LOAN_PREVIEW,
      debtFreeBefore: null,
      safeBefore: null,
      safeAfter: null,
      windows: [],
    },
  });
  fireEvent.click(dialog().getByRole("button", { name: "Add loan" }));
  await screen.findByText("You'd be debt-free on Wed, May 5, 2027.");
  expect(screen.queryByText(/Safe to spend/)).toBeNull();
});

test("a row the preview refuses shows under the list, with no confirm", async () => {
  await openAdd({
    ...ANSWERS,
    preview_loan: validation("payments", "Payment 1: enter a real date"),
  });
  fireEvent.click(dialog().getByRole("button", { name: "Add loan" }));
  await dialog().findByText("Payment 1: enter a real date");
  expect(screen.queryByText("Before you add this loan")).toBeNull();
  expect(calls("create_loan")).toEqual([]);
});
