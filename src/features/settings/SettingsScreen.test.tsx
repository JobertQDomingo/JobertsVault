import { invoke } from "@tauri-apps/api/core";
import { fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";
import type { Settings } from "@/bindings/Settings";
import { CATEGORIES, PAYMENT_METHODS } from "@/test/fixtures";
import { renderRoute, SETTINGS, type Answers } from "@/test/render";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

const ANSWERS: Answers = {
  get_app_version: "0.1.0",
  list_categories: CATEGORIES,
  list_payment_methods: PAYMENT_METHODS,
  update_settings: ({ settings }: { settings: Settings }) => ({
    ...settings,
    decimalsLocked: false,
  }),
};
// What update_settings receives: the whole set (DATA_MODEL §3.5).
const { decimalsLocked, ...STORED } = SETTINGS;

const saves = () =>
  vi
    .mocked(invoke)
    .mock.calls.filter(([c]) => c === "update_settings")
    .map(([, args]) => (args as { settings: Settings }).settings);
const radio = (name: string) => screen.getByRole("radio", { name });

test("General shows the stored settings", async () => {
  renderRoute("/settings", ANSWERS);
  expect(
    (
      (await screen.findByRole("textbox", {
        name: "Currency symbol",
      })) as HTMLInputElement
    ).value,
  ).toBe("₱");
  expect(radio("Before · ₱1.00").getAttribute("aria-checked")).toBe("true");
  expect(radio("Monday").getAttribute("aria-checked")).toBe("true");
  expect(radio("2").getAttribute("aria-checked")).toBe("true");
  expect(
    (
      screen.getByRole("combobox", {
        name: "Month starts on day",
      }) as HTMLInputElement
    ).value,
  ).toBe("1");
  expect(
    screen
      .getByRole("switch", { name: "Payment method field" })
      .getAttribute("aria-checked"),
  ).toBe("true");
});

test("each change saves the whole set straight away", async () => {
  renderRoute("/settings", ANSWERS);
  fireEvent.click(await screen.findByRole("radio", { name: "Sunday" }));
  await waitFor(() =>
    expect(saves()).toEqual([{ ...STORED, weekStart: "sunday" }]),
  );
  expect(await screen.findByText("Saved")).toBeTruthy();
  fireEvent.click(screen.getByRole("radio", { name: /^After · 1.00/ }));
  fireEvent.click(screen.getByRole("switch", { name: "Payment method field" }));
  await waitFor(() => expect(saves()).toHaveLength(3));
  expect(saves()[2]).toEqual({
    ...STORED,
    weekStart: "sunday",
    currencyPosition: "after",
    showPaymentMethod: false,
  });
});

test("month start day is picked from 1 to 28", async () => {
  renderRoute("/settings", ANSWERS);
  fireEvent.click(
    await screen.findByRole("combobox", { name: "Month starts on day" }),
  );
  expect(screen.queryByRole("option", { name: "29" })).toBeNull();
  fireEvent.click(screen.getByRole("option", { name: "25" }));
  await waitFor(() =>
    expect(saves()).toEqual([{ ...STORED, monthStartDay: 25 }]),
  );
});

test("the currency symbol saves on Enter, and the backend's message shows under it", async () => {
  renderRoute("/settings", {
    ...ANSWERS,
    update_settings: ({ settings }: { settings: Settings }) => {
      if (settings.currencySymbol.length > 5)
        throw {
          code: "validation",
          field: "currencySymbol",
          message: "Enter a symbol of 1 to 5 characters",
        };
      return { ...settings, decimalsLocked: false };
    },
  });
  const symbol = (await screen.findByRole("textbox", {
    name: "Currency symbol",
  })) as HTMLInputElement;
  symbol.focus();
  fireEvent.change(symbol, { target: { value: "PHPPHP" } });
  fireEvent.keyDown(symbol, { key: "Enter" });
  expect(
    await screen.findByText("Enter a symbol of 1 to 5 characters"),
  ).toBeTruthy();
  fireEvent.change(symbol, { target: { value: "PHP" } });
  fireEvent.blur(symbol);
  await waitFor(() =>
    expect(saves()[saves().length - 1]).toEqual({
      ...STORED,
      currencySymbol: "PHP",
    }),
  );
  await waitFor(() =>
    expect(
      screen.queryByText("Enter a symbol of 1 to 5 characters"),
    ).toBeNull(),
  );
});

test("decimal places are locked once amounts are saved, and say why", async () => {
  renderRoute("/settings", {
    ...ANSWERS,
    get_settings: { ...SETTINGS, decimalsLocked: true },
  });
  await waitFor(() =>
    expect((radio("0") as HTMLButtonElement).disabled).toBe(true),
  );
  // It looks disabled too (Radix marks only the items, not the group).
  expect(
    screen
      .getByRole("radiogroup", { name: "Decimal places" })
      .className.split(" "),
  ).toContain("opacity-50");
  expect(
    screen.getByText(
      "Locked because amounts are already saved. Changing it would change every stored amount.",
    ),
  ).toBeTruthy();
});

test("the tab is kept in the URL", async () => {
  const { router } = renderRoute("/settings?tab=methods", ANSWERS);
  expect(
    (await screen.findByRole("tab", { name: "Payment methods" })).getAttribute(
      "aria-selected",
    ),
  ).toBe("true");
  fireEvent.mouseDown(screen.getByRole("tab", { name: "Categories" }));
  await waitFor(() =>
    expect(router.state.location.search).toBe("?tab=categories"),
  );
});

test("a refused change doesn't take a quick second change down with it", async () => {
  let finishFirst: () => void = () => {};
  renderRoute("/settings", {
    ...ANSWERS,
    update_settings: ({ settings }: { settings: Settings }) => {
      if (settings.currencySymbol === "")
        return new Promise((_, reject) => {
          finishFirst = () =>
            reject({
              code: "validation",
              field: "currencySymbol",
              message: "Enter a symbol of 1 to 5 characters",
            });
        });
      return { ...settings, decimalsLocked: false };
    },
  });
  const symbol = (await screen.findByRole("textbox", {
    name: "Currency symbol",
  })) as HTMLInputElement;
  symbol.focus();
  fireEvent.change(symbol, { target: { value: "" } });
  fireEvent.blur(symbol);
  fireEvent.click(radio("Sunday"));
  await waitFor(() => expect(saves()).toHaveLength(1));
  finishFirst();
  await waitFor(() => expect(saves()).toHaveLength(2));
  expect(saves()[1]).toEqual({ ...STORED, weekStart: "sunday" });
  expect(
    await screen.findByText("Enter a symbol of 1 to 5 characters"),
  ).toBeTruthy();
});

test("the symbol shows as saved (trimmed), and Enter then leaving saves once", async () => {
  renderRoute("/settings", {
    ...ANSWERS,
    update_settings: ({ settings }: { settings: Settings }) => ({
      ...settings,
      currencySymbol: settings.currencySymbol.trim(),
      decimalsLocked: false,
    }),
  });
  const symbol = (await screen.findByRole("textbox", {
    name: "Currency symbol",
  })) as HTMLInputElement;
  symbol.focus();
  fireEvent.change(symbol, { target: { value: "$ " } });
  fireEvent.keyDown(symbol, { key: "Enter" });
  fireEvent.blur(symbol);
  await waitFor(() => expect(symbol.value).toBe("$"));
  fireEvent.blur(symbol);
  await new Promise((r) => setTimeout(r, 20));
  expect(saves()).toEqual([{ ...STORED, currencySymbol: "$ " }]);
});

test("General shows the version, and Check for updates says when it's up to date (UPD-3)", async () => {
  renderRoute("/settings", { ...ANSWERS, check_update: null });
  expect(await screen.findByText("0.1.0")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Check for updates" }));
  expect(await screen.findByText("You're up to date.")).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Update now" })).toBeNull();
});

test("a newer version is named, and Update now installs it (UPD-3)", async () => {
  renderRoute("/settings", {
    ...ANSWERS,
    check_update: "0.2.0",
    install_update: () => new Promise(() => {}),
  });
  fireEvent.click(
    await screen.findByRole("button", { name: "Check for updates" }),
  );
  expect(await screen.findByText("0.2.0 is out.")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Update now" }));
  expect(
    await screen.findByRole("button", { name: "Updating to 0.2.0…" }),
  ).toBeTruthy();
  expect(invoke).toHaveBeenCalledWith("install_update", undefined);
});

test("a refused update says why, and nothing changes (UPD-4)", async () => {
  const message =
    "Couldn't install the update: signature verification failed. Nothing was changed.";
  renderRoute("/settings", {
    ...ANSWERS,
    check_update: "0.2.0",
    install_update: () => {
      throw { code: "update", message };
    },
  });
  fireEvent.click(
    await screen.findByRole("button", { name: "Check for updates" }),
  );
  fireEvent.click(await screen.findByRole("button", { name: "Update now" }));
  expect(await screen.findByText(message)).toBeTruthy();
  expect(screen.getByRole("button", { name: "Update now" })).toBeTruthy();
});

test("a failed check says why", async () => {
  renderRoute("/settings", {
    ...ANSWERS,
    check_update: () => {
      throw { code: "update", message: "Couldn't check for updates: offline." };
    },
  });
  fireEvent.click(
    await screen.findByRole("button", { name: "Check for updates" }),
  );
  expect(
    await screen.findByText("Couldn't check for updates: offline."),
  ).toBeTruthy();
});

test("the automatic updates switch saves at once (UPD-3)", async () => {
  renderRoute("/settings", ANSWERS);
  fireEvent.click(
    await screen.findByRole("switch", { name: "Automatic updates" }),
  );
  await waitFor(() =>
    expect(saves()).toEqual([{ ...STORED, autoUpdateEnabled: false }]),
  );
});
