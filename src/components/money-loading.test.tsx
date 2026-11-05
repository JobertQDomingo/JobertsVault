import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { AmountInput } from "./AmountInput";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: () => new Promise(() => {}),
}));

test("the amount waits for the currency settings before taking input", () => {
  render(
    <QueryClientProvider client={new QueryClient()}>
      <AmountInput aria-label="Amount" value={null} onChange={() => {}} />
    </QueryClientProvider>,
  );
  expect((screen.getByLabelText("Amount") as HTMLInputElement).disabled).toBe(
    true,
  );
});
