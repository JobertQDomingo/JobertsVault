import { invoke } from "@tauri-apps/api/core";
import {
  focusManager,
  QueryClient,
  QueryClientProvider,
} from "@tanstack/react-query";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";
import App from "./App";
import { createQueryClient } from "./lib/queryClient";
import { queryKeys } from "./lib/queryKeys";
import { SETTINGS } from "./test/render";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const FAILED = {
  status: "failed",
  message: "This database was made by a newer version of Jobert's Vault.",
  databasePath:
    "C:\\Users\\Jobert\\AppData\\Roaming\\com.jobertsvault.app\\jobertsvault.db",
};

/** Answers get_startup_status with `status`, and the commands in `answers`; the rest get null. */
function renderApp(
  status: object,
  answers: Record<string, () => unknown> = {},
  client = new QueryClient({ defaultOptions: { queries: { retry: false } } }),
) {
  vi.mocked(invoke).mockImplementation((command: string) =>
    command in answers
      ? Promise.resolve().then(answers[command])
      : Promise.resolve(command === "get_startup_status" ? status : null),
  );
  render(
    <QueryClientProvider client={client}>
      <App />
    </QueryClientProvider>,
  );
}

beforeEach(() => vi.mocked(invoke).mockReset());

test("when startup works, the app shell opens on the dashboard", async () => {
  renderApp({ status: "ok" });
  const title = await screen.findByRole("heading", { name: "Dashboard" });
  expect(title.closest(".glass")).toBeTruthy();
  expect(
    screen.getByRole("navigation", { name: "Main" }).closest(".glass"),
  ).toBeTruthy();
});

test("when startup fails, the error screen shows the error and the database path", async () => {
  renderApp(FAILED);
  expect(
    await screen.findByRole("heading", {
      name: "Jobert's Vault couldn't start",
    }),
  ).toBeTruthy();
  expect(screen.getByText(FAILED.message)).toBeTruthy();
  expect(screen.getByText(FAILED.databasePath)).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Open data folder" }));
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith("open_data_folder", undefined),
  );
});

test("if the data folder can't be opened, a toast says why", async () => {
  renderApp(FAILED);
  const button = await screen.findByRole("button", {
    name: "Open data folder",
  });
  vi.mocked(invoke).mockRejectedValueOnce({
    code: "io",
    message: "Couldn't open Explorer.",
  });
  fireEvent.click(button);
  expect(await screen.findByText("Couldn't open Explorer.")).toBeTruthy();
});

test("if the status itself can't be read, the error screen still shows, and it isn't asked again", async () => {
  vi.mocked(invoke).mockRejectedValueOnce("IPC is broken");
  render(
    <QueryClientProvider client={createQueryClient()}>
      <App />
    </QueryClientProvider>,
  );
  expect(
    await screen.findByRole("heading", {
      name: "Jobert's Vault couldn't start",
    }),
  ).toBeTruthy();
  expect(screen.getByText("IPC is broken")).toBeTruthy();
  expect(screen.queryByText("Database")).toBeNull();
  focusManager.setFocused(false);
  focusManager.setFocused(true);
  expect(invoke).toHaveBeenCalledOnce();
});

const OK = { status: "ok" };
const autoUpdate = (on: boolean) => () => ({
  ...SETTINGS,
  autoUpdateEnabled: on,
});
const never = () => new Promise(() => {});

test("with automatic updates on, a newer version is installed before the app opens (UPD-1)", async () => {
  renderApp(OK, {
    get_settings: autoUpdate(true),
    check_update: () => "0.2.0",
    install_update: never,
  });
  expect(
    await screen.findByText("Updating Jobert's Vault to 0.2.0…"),
  ).toBeTruthy();
  expect(invoke).toHaveBeenCalledWith("install_update", undefined);
  expect(screen.queryByRole("navigation", { name: "Main" })).toBeNull();
});

test("a failed check opens the app with no error (UPD-2)", async () => {
  renderApp(OK, {
    get_settings: autoUpdate(true),
    check_update: () => {
      throw { code: "update", message: "Couldn't check for updates: offline." };
    },
  });
  expect(
    await screen.findByRole("heading", { name: "Dashboard" }),
  ).toBeTruthy();
  expect(screen.queryByText(/Couldn't check/)).toBeNull();
  expect(invoke).not.toHaveBeenCalledWith("install_update", undefined);
});

test("when up to date, the app opens", async () => {
  renderApp(OK, { get_settings: autoUpdate(true), check_update: () => null });
  expect(
    await screen.findByRole("heading", { name: "Dashboard" }),
  ).toBeTruthy();
  expect(invoke).not.toHaveBeenCalledWith("install_update", undefined);
});

test("with automatic updates off, the app never checks by itself (UPD-3)", async () => {
  renderApp(OK, { get_settings: autoUpdate(false) });
  expect(
    await screen.findByRole("heading", { name: "Dashboard" }),
  ).toBeTruthy();
  expect(invoke).not.toHaveBeenCalledWith("check_update", undefined);
});

test("a failed install says why, then opens the app (UPD-4)", async () => {
  const message =
    "Couldn't install the update: signature verification failed. Nothing was changed.";
  renderApp(OK, {
    get_settings: autoUpdate(true),
    check_update: () => "0.2.0",
    install_update: () => {
      throw { code: "update", message };
    },
  });
  expect(await screen.findByText(message)).toBeTruthy();
  expect(
    await screen.findByRole("heading", { name: "Dashboard" }),
  ).toBeTruthy();
});

test("a check that answers within 150 ms never shows Checking for updates…", async () => {
  // Both delays start together, so the 100 ms answer always comes before the 150 ms notice.
  const later = (ms: number, value: unknown) => () =>
    new Promise((resolve) => setTimeout(() => resolve(value), ms));
  const seen: string[] = [];
  const watcher = new MutationObserver(() =>
    seen.push(document.body.textContent ?? ""),
  );
  watcher.observe(document.body, { childList: true, subtree: true });
  renderApp(OK, {
    get_settings: autoUpdate(true),
    check_update: later(100, null),
  });
  expect(
    await screen.findByRole("heading", { name: "Dashboard" }),
  ).toBeTruthy();
  watcher.disconnect();
  expect(seen.some((text) => text.includes("Checking for updates…"))).toBe(
    false,
  );
});

test("a check that takes longer than 150 ms shows Checking for updates…", async () => {
  renderApp(OK, { get_settings: autoUpdate(true), check_update: never });
  expect(await screen.findByText("Checking for updates…")).toBeTruthy();
});

test("turning automatic updates on later doesn't start an update mid-session", async () => {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  renderApp(OK, { get_settings: autoUpdate(false) }, client);
  expect(
    await screen.findByRole("heading", { name: "Dashboard" }),
  ).toBeTruthy();
  act(() => client.setQueryData(queryKeys.settings, autoUpdate(true)()));
  expect(screen.getByRole("heading", { name: "Dashboard" })).toBeTruthy();
  expect(invoke).not.toHaveBeenCalledWith("check_update", undefined);
});
