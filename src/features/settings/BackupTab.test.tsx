import { invoke } from "@tauri-apps/api/core";
import { fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";
import { renderRoute, type Answers } from "@/test/render";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

const ANSWERS: Answers = {
  backup_now: { fileName: "jobertsvault-2026-11-07_0930.db" },
  open_backup_folder: null,
};
const called = (command: string) =>
  vi.mocked(invoke).mock.calls.filter(([c]) => c === command).length;

test("the Backup tab is kept in the address", async () => {
  renderRoute("/settings?tab=backup", ANSWERS);
  expect(
    (await screen.findByRole("tab", { name: "Backup" })).getAttribute(
      "aria-selected",
    ),
  ).toBe("true");
});

test("Back up now names the copy and opens its folder (BAK-1)", async () => {
  renderRoute("/settings?tab=backup", ANSWERS);
  fireEvent.click(await screen.findByRole("button", { name: "Back up now" }));
  expect(
    await screen.findByText("Backup saved as jobertsvault-2026-11-07_0930.db"),
  ).toBeTruthy();
  expect(called("backup_now")).toBe(1);

  fireEvent.click(screen.getByRole("button", { name: "Open folder" }));
  await waitFor(() => expect(called("open_backup_folder")).toBe(1));
});

test("a failed backup says why", async () => {
  renderRoute("/settings?tab=backup", {
    ...ANSWERS,
    backup_now: () => {
      throw { code: "io", message: "Couldn't save the backup. Disk full." };
    },
  });
  fireEvent.click(await screen.findByRole("button", { name: "Back up now" }));
  expect(
    await screen.findByText("Couldn't save the backup. Disk full."),
  ).toBeTruthy();
});
