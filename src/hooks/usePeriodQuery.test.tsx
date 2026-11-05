import { act, renderHook } from "@testing-library/react";
import type { ReactNode } from "react";
import { createMemoryRouter, RouterProvider, useLocation } from "react-router";
import { expect, test } from "vitest";
import { usePeriodQuery } from "./usePeriodQuery";

function setup(url: string) {
  let location = "";
  const result = {
    current: null as unknown as ReturnType<typeof usePeriodQuery>,
  };
  function Probe() {
    result.current = usePeriodQuery();
    const l = useLocation();
    location = l.search;
    return null;
  }
  const router = createMemoryRouter([{ path: "/", element: <Probe /> }], {
    initialEntries: [url],
  });
  renderHook(() => null, {
    wrapper: ({ children }: { children: ReactNode }) => (
      <>
        <RouterProvider router={router} />
        {children}
      </>
    ),
  });
  return { result, location: () => location };
}

test("no period in the URL means this month", () => {
  expect(setup("/").result.current[0]).toEqual({ kind: "month", anchor: null });
});

test("the URL's period is read", () => {
  expect(setup("/?period=week&anchor=2026-10-26").result.current[0]).toEqual({
    kind: "week",
    anchor: "2026-10-26",
  });
  expect(
    setup("/?period=custom&start=2026-10-01&end=2026-10-10").result.current[0],
  ).toEqual({ kind: "custom", start: "2026-10-01", end: "2026-10-10" });
});

test("a bad URL falls back to this month", () => {
  for (const url of [
    "/?period=bogus",
    "/?period=custom&start=2026-10-01",
    "/?period=custom&start=2026-10-10&end=2026-10-01",
  ])
    expect(setup(url).result.current[0], url).toEqual({
      kind: "month",
      anchor: null,
    });
  expect(setup("/?period=day&anchor=2026-02-30").result.current[0]).toEqual({
    kind: "day",
    anchor: null,
  });
});

test("changing the period keeps the other parameters", () => {
  const { result, location } = setup("/?period=month&search=coffee");
  act(() =>
    result.current[1]({
      kind: "custom",
      start: "2026-10-01",
      end: "2026-10-10",
    }),
  );
  const params = new URLSearchParams(location());
  expect(Object.fromEntries(params)).toEqual({
    period: "custom",
    start: "2026-10-01",
    end: "2026-10-10",
    search: "coffee",
  });
  act(() => result.current[1]({ kind: "week", anchor: null }));
  expect(Object.fromEntries(new URLSearchParams(location()))).toEqual({
    period: "week",
    search: "coffee",
  });
});
