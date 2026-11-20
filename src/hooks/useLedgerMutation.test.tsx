import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { act, render, renderHook, screen } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, expect, test, vi } from "vitest";
import type { ActiveFocus } from "@/bindings/ActiveFocus";
import type { GoalMove } from "@/bindings/GoalMove";
import { notify, Toaster } from "@/components/ui/toast";
import { queryKeys } from "@/lib/queryKeys";
import { activeFocus, CATEGORIES, CHECKIN_STATUS } from "@/test/fixtures";
import { SETTINGS } from "@/test/render";
import { useLedgerMutation } from "./useLedgerMutation";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

const save = async (goalsMoved: GoalMove[], client = new QueryClient()) => {
  render(<Toaster />);
  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  );
  const { result } = renderHook(
    () => useLedgerMutation(async () => ({ id: 7, goalsMoved })),
    { wrapper },
  );
  await act(() => result.current.mutateAsync(undefined));
};

test("a save that moved goals says so, one line per goal (GOL-7)", async () => {
  await save([
    { id: 1, name: "Emergency fund", from: "2027-05-05", to: "2027-07-05" },
    { id: 2, name: "Japan trip", from: "2027-02-03", to: "2027-03-03" },
  ]);
  const toast = await screen.findByText(/^Emergency fund now aims/);
  // Shown on two lines: the `\n` stays, and the toast keeps line breaks.
  expect(toast.textContent).toBe(
    "Emergency fund now aims for Jul 5, 2027 (was May 5, 2027), so it stays realistic.\n" +
      "Japan trip now aims for Mar 3, 2027 (was Feb 3, 2027), so it stays realistic.",
  );
  expect(toast.closest(".whitespace-pre-line")).not.toBeNull();
});

test("a save that moved no goal says nothing", async () => {
  await save([]);
  expect(screen.queryByText(/now aims for/)).toBeNull();
});

const dining = (over: Partial<ActiveFocus>) =>
  activeFocus({
    kind: "category_cap",
    categoryId: 2,
    target: 150000,
    ...over,
  });

/** A save with `before` cached and `after` what `get_vault_status` answers next. */
async function nudge(before: ActiveFocus[], after: ActiveFocus[]) {
  const warning = vi.spyOn(notify, "warning");
  const client = new QueryClient();
  client.setQueryData(queryKeys.settings, SETTINGS);
  client.setQueryData(queryKeys.categories, CATEGORIES);
  client.setQueryData(queryKeys.vaultStatus, {
    ...CHECKIN_STATUS,
    focuses: before,
  });
  vi.mocked(invoke).mockImplementation(async (cmd) => {
    if (cmd === "get_vault_status")
      return { ...CHECKIN_STATUS, focuses: after };
    throw new Error(`unexpected ${cmd}`);
  });
  await save([], client);
  const said = warning.mock.calls.map((c) => c[0]);
  warning.mockRestore();
  return said;
}

test("a save that takes a cap to 80% of its target warns once (F18)", async () => {
  expect(
    await nudge(
      [dining({ actual: 100000, percent: 66, state: "ok" })],
      [dining({ actual: 124000, percent: 82, state: "near" })],
    ),
  ).toEqual(["Dining Out focus: ₱1,240.00 of ₱1,500.00."]);
});

test("a save that takes a cap past its target warns, and a spending cap says Spending (F18)", async () => {
  const spending = (over: Partial<ActiveFocus>) =>
    activeFocus({
      kind: "spending_cap",
      target: 150000,
      start: "2026-11-01",
      end: "2026-11-30",
      ...over,
    });
  expect(
    await nudge(
      [spending({ actual: 124000, percent: 82, state: "near" })],
      [spending({ actual: 172000, percent: 114, state: "over" })],
    ),
  ).toEqual(["Spending focus: ₱1,720.00, past your ₱1,500.00 target."]);
});

test("a save that doesn't raise a cap's state says nothing (F18 step 4)", async () => {
  expect(
    await nudge(
      [dining({ actual: 124000, percent: 82, state: "near" })],
      [dining({ actual: 130000, percent: 86, state: "near" })],
    ),
  ).toEqual([]);
  expect(
    await nudge(
      [dining({ actual: 172000, percent: 114, state: "over" })],
      [dining({ actual: 100000, percent: 66, state: "ok" })],
    ),
  ).toEqual([]);
});

test("a card-level focus has no focus toast: the card's levels warn (F18 step 4)", async () => {
  const card = (actual: number) =>
    activeFocus({ kind: "card_level", accountId: 4, target: 400000, actual });
  // A cap's state is all it compares, and a card level has none.
  expect(await nudge([card(100000)], [card(450000)])).toEqual([]);
  expect(invoke).not.toHaveBeenCalled();
});
