import { invoke } from "@tauri-apps/api/core";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";
import App from "./App";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const FAILED = {
  status: "failed",
  message: "This database was made by a newer version of Jobert's Vault.",
  databasePath:
    "C:\\Users\\Jobert\\AppData\\Roaming\\com.jobertsvault.app\\jobertsvault.db",
};

function renderApp(status: object) {
  vi.mocked(invoke).mockImplementation((command: string) =>
    Promise.resolve(command === "get_startup_status" ? status : null),
  );
  render(
    <QueryClientProvider
      client={
        new QueryClient({ defaultOptions: { queries: { retry: false } } })
      }
    >
      <App />
    </QueryClientProvider>,
  );
}

beforeEach(() => vi.mocked(invoke).mockReset());

test("when startup works, the app shows its name on glass", async () => {
  renderApp({ status: "ok" });
  const title = await screen.findByRole("heading", { name: "Jobert's Vault" });
  expect(title.closest(".glass")).toBeTruthy();
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
