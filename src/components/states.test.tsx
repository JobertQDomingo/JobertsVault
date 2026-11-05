import { act, fireEvent, render, screen } from "@testing-library/react";
import { Receipt } from "lucide-react";
import { afterEach, expect, test, vi } from "vitest";
import { EmptyState } from "./EmptyState";
import { ErrorState } from "./ErrorState";
import { Loading, Skeleton } from "./Loading";
import { Button } from "./ui/button";

afterEach(() => vi.useRealTimers());

test("an empty state has an icon, one line and one action", () => {
  const onAdd = vi.fn();
  render(
    <EmptyState
      icon={Receipt}
      message="No transactions yet. Press Ctrl+N to add your first one."
      action={<Button onClick={onAdd}>Add transaction</Button>}
    />,
  );
  expect(
    screen.getByText(
      "No transactions yet. Press Ctrl+N to add your first one.",
    ),
  ).toBeTruthy();
  expect(document.querySelector("svg[aria-hidden=true].size-6")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Add transaction" }));
  expect(onAdd).toHaveBeenCalledOnce();
});

test("a skeleton appears only once loading takes 150 ms", () => {
  vi.useFakeTimers();
  render(
    <Loading>
      <Skeleton className="h-4" />
    </Loading>,
  );
  expect(screen.getByRole("status")).toBeTruthy();
  act(() => vi.advanceTimersByTime(149));
  expect(document.querySelector(".bg-wash")).toBeNull();
  act(() => vi.advanceTimersByTime(1));
  expect(document.querySelector(".bg-wash")).toBeTruthy();
});

test("an error says where the details are and offers Retry", () => {
  const onRetry = vi.fn();
  render(<ErrorState onRetry={onRetry} />);
  expect(screen.getByText(/The details are in the error log/)).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Retry" }));
  expect(onRetry).toHaveBeenCalledOnce();
});

test("the loading region says Loading in text a screen reader can read", () => {
  render(<Loading>{null}</Loading>);
  expect(screen.getByRole("status").textContent).toBe("Loading");
});

test("Retry shows that it's running", () => {
  render(<ErrorState onRetry={() => {}} pending />);
  expect(
    (screen.getByRole("button", { name: "Retry" }) as HTMLButtonElement)
      .disabled,
  ).toBe(true);
});
