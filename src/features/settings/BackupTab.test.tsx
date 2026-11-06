import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";
import { PERIOD } from "@/test/fixtures";
import { renderRoute, type Answers } from "@/test/render";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), save: vi.fn() }));

beforeEach(() => {
  vi.mocked(invoke).mockReset();
  vi.mocked(open).mockReset();
  vi.mocked(save).mockReset();
});

const ANSWERS: Answers = {
  backup_now: { fileName: "jobertsvault-2026-11-07_0930.db" },
  open_backup_folder: null,
  inspect_backup: { modified: "2026-10-20T09:30" },
  restore_backup: null,
  get_period: PERIOD,
  export_csv: { rows: 12 },
};
const CHOSEN = String.raw`D:\Backups\jobertsvault-2026-10-20_0930.db`;
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

test("Restore asks with the backup's date, then restores and goes to the dashboard (BAK-4)", async () => {
  vi.mocked(open).mockResolvedValue(CHOSEN);
  const { router } = renderRoute("/settings?tab=backup", ANSWERS);
  fireEvent.click(
    await screen.findByRole("button", { name: "Restore from backup…" }),
  );
  const dialog = await screen.findByRole("dialog", {
    name: "Restore this backup?",
  });
  expect(dialog.textContent).toContain(
    "This replaces all current data with the backup from Oct 20, 2026 9:30 AM. A safety copy of your current data will be saved first.",
  );
  expect(vi.mocked(invoke)).toHaveBeenCalledWith("inspect_backup", {
    path: CHOSEN,
  });
  expect(called("restore_backup")).toBe(0);

  fireEvent.click(screen.getByRole("button", { name: "Restore" }));
  expect(await screen.findByText("Backup restored")).toBeTruthy();
  expect(vi.mocked(invoke)).toHaveBeenCalledWith("restore_backup", {
    path: CHOSEN,
  });
  expect(router.state.location.pathname).toBe("/");
});

test("closing the picker does nothing", async () => {
  vi.mocked(open).mockResolvedValue(null);
  renderRoute("/settings?tab=backup", ANSWERS);
  fireEvent.click(
    await screen.findByRole("button", { name: "Restore from backup…" }),
  );
  await waitFor(() => expect(open).toHaveBeenCalled());
  expect(called("inspect_backup")).toBe(0);
  expect(screen.queryByRole("dialog")).toBeNull();
});

test("a refused file says why, and nothing is restored", async () => {
  vi.mocked(open).mockResolvedValue(String.raw`D:\notes.db`);
  const message =
    "This file isn't a Jobert's Vault backup, so nothing was changed.";
  renderRoute("/settings?tab=backup", {
    ...ANSWERS,
    inspect_backup: () => {
      throw { code: "validation", field: "path", message };
    },
  });
  fireEvent.click(
    await screen.findByRole("button", { name: "Restore from backup…" }),
  );
  expect(await screen.findByText(message)).toBeTruthy();
  expect(screen.queryByRole("dialog")).toBeNull();
  expect(called("restore_backup")).toBe(0);
});

test("Cancel restores nothing", async () => {
  vi.mocked(open).mockResolvedValue(CHOSEN);
  const { router } = renderRoute("/settings?tab=backup", ANSWERS);
  fireEvent.click(
    await screen.findByRole("button", { name: "Restore from backup…" }),
  );
  await screen.findByRole("dialog");
  fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  expect(called("restore_backup")).toBe(0);
  expect(router.state.location.pathname).toBe("/settings");
});

test("Export saves this month so far where the user chooses (BAK-6)", async () => {
  vi.mocked(save).mockResolvedValue(
    String.raw`C:\Users\me\Documents\november.csv`,
  );
  renderRoute("/settings?tab=backup", ANSWERS);
  const value = (name: string) =>
    (screen.getByRole("textbox", { name }) as HTMLInputElement).value;
  // From get_period: the month's first day, and today.
  await waitFor(() => expect(value("From")).toBe("11/01/2026"));
  expect(value("To")).toBe("11/05/2026");

  fireEvent.click(screen.getByRole("button", { name: "Export…" }));
  expect(
    await screen.findByText("Exported 12 transactions to november.csv"),
  ).toBeTruthy();
  expect(save).toHaveBeenCalledWith(
    expect.objectContaining({
      defaultPath: "jobertsvault-2026-11-01-to-2026-11-05.csv",
    }),
  );
  expect(vi.mocked(invoke)).toHaveBeenCalledWith("export_csv", {
    range: { start: "2026-11-01", end: "2026-11-05" },
    path: String.raw`C:\Users\me\Documents\november.csv`,
  });
});

test("closing the save dialog exports nothing", async () => {
  vi.mocked(save).mockResolvedValue(null);
  renderRoute("/settings?tab=backup", ANSWERS);
  await screen.findByRole("textbox", { name: "From" });
  await waitFor(() =>
    expect(
      (screen.getByRole("button", { name: "Export…" }) as HTMLButtonElement)
        .disabled,
    ).toBe(false),
  );
  fireEvent.click(screen.getByRole("button", { name: "Export…" }));
  await waitFor(() => expect(save).toHaveBeenCalled());
  expect(called("export_csv")).toBe(0);
});

test("a refused export says why", async () => {
  vi.mocked(save).mockResolvedValue(String.raw`C:\out.csv`);
  renderRoute("/settings?tab=backup", {
    ...ANSWERS,
    export_csv: () => {
      throw {
        code: "validation",
        field: "end",
        message: "The end date can't be before the start date",
      };
    },
  });
  await waitFor(() =>
    expect(
      (screen.getByRole("button", { name: "Export…" }) as HTMLButtonElement)
        .disabled,
    ).toBe(false),
  );
  fireEvent.click(screen.getByRole("button", { name: "Export…" }));
  expect(
    await screen.findByText("The end date can't be before the start date"),
  ).toBeTruthy();
});

test("dates out of order are caught before the save dialog", async () => {
  renderRoute("/settings?tab=backup", ANSWERS);
  const to = (await screen.findByRole("textbox", {
    name: "To",
  })) as HTMLInputElement;
  await waitFor(() => expect(to.value).toBe("11/05/2026"));
  fireEvent.change(to, { target: { value: "10/01/2026" } });
  fireEvent.blur(to);
  fireEvent.click(screen.getByRole("button", { name: "Export…" }));
  expect(
    await screen.findByText("Enter a From date and a To date on or after it."),
  ).toBeTruthy();
  expect(save).not.toHaveBeenCalled();
  expect(called("export_csv")).toBe(0);
});
