import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, render, renderHook, screen } from "@testing-library/react";
import type { ReactNode } from "react";
import { expect, test } from "vitest";
import type { GoalMove } from "@/bindings/GoalMove";
import { Toaster } from "@/components/ui/toast";
import { useLedgerMutation } from "./useLedgerMutation";

const save = async (goalsMoved: GoalMove[]) => {
  render(<Toaster />);
  const client = new QueryClient();
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
