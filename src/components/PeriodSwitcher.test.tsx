import { invoke } from "@tauri-apps/api/core";
import { QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { createMemoryRouter, RouterProvider } from "react-router";
import { beforeEach, expect, test, vi } from "vitest";
import type { PeriodQuery } from "@/bindings/PeriodQuery";
import type { PeriodView } from "@/bindings/PeriodView";
import { createQueryClient } from "@/lib/queryClient";
import { queryKeys } from "@/lib/queryKeys";
import { SETTINGS } from "@/test/render";
import { PeriodSwitcher } from "./PeriodSwitcher";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const TODAY = "2026-11-05";
const view = (over: Partial<PeriodView>): PeriodView => ({
  start: "2026-11-01",
  end: "2026-11-30",
  previous: { kind: "month", anchor: "2026-10-01" },
  next: null,
  todayQuery: { kind: "month", anchor: null },
  isCurrent: true,
  today: TODAY,
  ...over,
});

// get_period as the backend answers it for the periods these tests visit.
function periodFor(query: PeriodQuery): PeriodView {
  if (query.kind === "month" && query.anchor === "2026-10-01")
    return view({
      start: "2026-10-01",
      end: "2026-10-31",
      previous: { kind: "month", anchor: "2026-09-01" },
      next: { kind: "month", anchor: "2026-11-01" },
      isCurrent: false,
    });
  if (query.kind === "week")
    return view({
      start: "2026-10-26",
      end: "2026-11-01",
      previous: { kind: "week", anchor: "2026-10-19" },
      next: { kind: "week", anchor: "2026-11-02" },
      todayQuery: { kind: "week", anchor: null },
      isCurrent: false,
    });
  if (query.kind === "custom")
    return view({
      start: query.start,
      end: query.end,
      previous: null,
      todayQuery: query,
    });
  return view({});
}

let router: ReturnType<typeof createMemoryRouter>;

function renderSwitcher(url = "/") {
  vi.mocked(invoke).mockImplementation((command: string, args?: unknown) =>
    Promise.resolve(
      command === "get_period"
        ? periodFor((args as { query: PeriodQuery }).query)
        : null,
    ),
  );
  const client = createQueryClient();
  client.setQueryData(queryKeys.settings, SETTINGS);
  router = createMemoryRouter([{ path: "/", element: <PeriodSwitcher /> }], {
    initialEntries: [url],
  });
  render(
    <QueryClientProvider client={client}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  );
}

const params = () =>
  Object.fromEntries(new URLSearchParams(router.state.location.search));
const button = (name: string) =>
  screen.getByRole("button", { name }) as HTMLButtonElement;

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

test("this month: its label, ◀ enabled, ▶ and Today disabled", async () => {
  renderSwitcher();
  expect(await screen.findByText("November 2026")).toBeTruthy();
  expect(button("Previous period").disabled).toBe(false);
  expect(button("Next period").disabled).toBe(true);
  expect(button("Today").disabled).toBe(true);
});

test("◀ goes to the previous period the backend named", async () => {
  renderSwitcher();
  await screen.findByText("November 2026");
  fireEvent.click(button("Previous period"));
  await screen.findByText("October 2026");
  expect(params()).toEqual({ period: "month", anchor: "2026-10-01" });
  expect(button("Next period").disabled).toBe(false);
});

test("Today goes back to the period containing today", async () => {
  renderSwitcher("/?period=month&anchor=2026-10-01");
  await screen.findByText("October 2026");
  fireEvent.click(button("Today"));
  await screen.findByText("November 2026");
  expect(params()).toEqual({ period: "month" });
});

test("switching to Week keeps the place you were looking at", async () => {
  renderSwitcher("/?period=month&anchor=2026-10-01");
  await screen.findByText("October 2026");
  fireEvent.click(screen.getByRole("radio", { name: "Week" }));
  await screen.findByText("Oct 26 – Nov 1, 2026");
  expect(params()).toEqual({ period: "week", anchor: "2026-10-01" });
});

test("Custom picks a range without future days", async () => {
  renderSwitcher();
  await screen.findByText("November 2026");
  fireEvent.click(screen.getByRole("radio", { name: "Custom" }));
  const day = (name: RegExp) => screen.getByRole("button", { name });
  expect((day(/November 6(th)?,/) as HTMLButtonElement).disabled).toBe(true);
  fireEvent.click(day(/November 2(nd)?,/));
  fireEvent.click(day(/November 4(th)?,/));
  fireEvent.click(button("Apply"));
  await screen.findByText("Nov 2 – Nov 4, 2026");
  expect(params()).toEqual({
    period: "custom",
    start: "2026-11-02",
    end: "2026-11-04",
  });
  await waitFor(() => expect(screen.queryByRole("grid")).toBeNull());
});

test("clicking Custom again reopens the picker on the chosen range", async () => {
  renderSwitcher("/?period=custom&start=2026-11-02&end=2026-11-04");
  await screen.findByText("Nov 2 – Nov 4, 2026");
  fireEvent.click(screen.getByRole("radio", { name: "Custom" }));
  const day = screen.getByRole("button", { name: /November 3(rd)?,/ });
  expect(day.closest("[data-selected]")).toBeTruthy();
  expect(button("Apply").disabled).toBe(false);
});

test("while the next period loads, the old label stays", async () => {
  renderSwitcher();
  await screen.findByText("November 2026");
  vi.mocked(invoke).mockReturnValue(new Promise(() => {}));
  fireEvent.click(button("Previous period"));
  await waitFor(() =>
    expect(params()).toEqual({ period: "month", anchor: "2026-10-01" }),
  );
  expect(screen.getByText("November 2026")).toBeTruthy();
});
