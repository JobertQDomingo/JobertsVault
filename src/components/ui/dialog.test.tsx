import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { useState } from "react";
import { expect, test, vi } from "vitest";
import { Button } from "./button";
import { ConfirmDialog, Dialog, DialogContent, DialogFooter } from "./dialog";
import { Menu, MenuContent, MenuItem, MenuTrigger } from "./menu";
import { notify, Toaster } from "./toast";

test("a dialog is named by its title and Esc closes it", () => {
  const onOpenChange = vi.fn();
  render(
    <Dialog open onOpenChange={onOpenChange}>
      <DialogContent title="Add transaction">
        <input aria-label="Amount" />
        <DialogFooter>
          <Button>Cancel</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>,
  );
  expect(screen.getByRole("dialog", { name: "Add transaction" })).toBeTruthy();
  fireEvent.keyDown(screen.getByLabelText("Amount"), { key: "Escape" });
  expect(onOpenChange).toHaveBeenCalledWith(false);
});

test("a confirm dialog's Cancel doesn't act, its action does", () => {
  const onConfirm = vi.fn();
  const onOpenChange = vi.fn();
  render(
    <ConfirmDialog
      open
      onOpenChange={onOpenChange}
      title="Delete this transaction?"
      message="This can't be undone."
      confirmLabel="Delete"
      destructive
      onConfirm={onConfirm}
    />,
  );
  expect(screen.getByText("This can't be undone.")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
  expect(onOpenChange).toHaveBeenCalledWith(false);
  expect(onConfirm).not.toHaveBeenCalled();
  const action = screen.getByRole("button", { name: "Delete" });
  expect(action.className).toContain("bg-danger");
  fireEvent.click(action);
  expect(onConfirm).toHaveBeenCalledOnce();
});

test("toasts show their message, and an error can be closed", async () => {
  render(<Toaster />);
  act(() => {
    notify.success("Saved");
    notify.error("Something went wrong with the database.");
  });
  expect(await screen.findByText("Saved")).toBeTruthy();
  const error = await screen.findByText(
    "Something went wrong with the database.",
  );
  const toast = error.closest("[data-sonner-toast]") as HTMLElement;
  expect(toast.querySelector("[data-close-button]")).toBeTruthy();
});

test("toasts use the app font, not sonner's system font", async () => {
  render(<Toaster />);
  act(() => {
    notify.success("Saved");
  });
  await screen.findByText("Saved");
  const toaster = document.querySelector(
    "[data-sonner-toaster]",
  ) as HTMLElement;
  expect(toaster.style.fontFamily).toBe("inherit");
});

test("a confirm dialog opens on Cancel, and confirming closes it", () => {
  const onConfirm = vi.fn();
  const onOpenChange = vi.fn();
  render(
    <ConfirmDialog
      open
      onOpenChange={onOpenChange}
      title="Archive Old Wallet?"
      message="It will be hidden from lists but its history stays."
      confirmLabel="Archive"
      onConfirm={onConfirm}
    />,
  );
  expect(document.activeElement?.textContent).toBe("Cancel");
  fireEvent.click(screen.getByRole("button", { name: "Archive" }));
  expect(onOpenChange).toHaveBeenCalledWith(false);
  expect(onConfirm).toHaveBeenCalledOnce();
});

function Opener() {
  const [open, setOpen] = useState(false);
  return (
    <>
      <Button onClick={() => setOpen(true)}>Edit</Button>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent title="Edit transaction">
          <input aria-label="Amount" />
        </DialogContent>
      </Dialog>
    </>
  );
}

test("closing a dialog opened from state puts focus back on the button that opened it (A11Y-01)", async () => {
  render(<Opener />);
  const edit = screen.getByRole("button", { name: "Edit" });
  edit.focus();
  fireEvent.click(edit);
  const amount = await screen.findByLabelText("Amount");
  await waitFor(() => expect(document.activeElement).toBe(amount));
  fireEvent.keyDown(amount, { key: "Escape" });
  expect(screen.queryByRole("dialog")).toBeNull();
  // Radix returns focus a tick after the dialog closes.
  await waitFor(() =>
    expect(document.activeElement).toBe(
      screen.getByRole("button", { name: "Edit" }),
    ),
  );
});

function FromMenu() {
  const [open, setOpen] = useState(false);
  return (
    <>
      <Menu>
        <MenuTrigger asChild>
          <Button aria-label="More actions">⋯</Button>
        </MenuTrigger>
        <MenuContent>
          <MenuItem onSelect={() => setOpen(true)}>Archive</MenuItem>
        </MenuContent>
      </Menu>
      <ConfirmDialog
        open={open}
        onOpenChange={setOpen}
        title="Archive Old Wallet?"
        message="It will be hidden from lists."
        confirmLabel="Archive"
        onConfirm={() => {}}
      />
    </>
  );
}

test("a confirm opened from a ⋯ menu item puts focus back on the ⋯ button (A11Y-01)", async () => {
  render(<FromMenu />);
  const more = screen.getByRole("button", { name: "More actions" });
  more.focus();
  fireEvent.keyDown(more, { key: "Enter" });
  const item = await screen.findByRole("menuitem", { name: "Archive" });
  await waitFor(() => expect(document.activeElement).toBe(item));
  fireEvent.keyDown(item, { key: "Enter" });
  await screen.findByRole("dialog", { name: "Archive Old Wallet?" });
  fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
  expect(screen.queryByRole("dialog")).toBeNull();
  // Radix returns focus a tick after the dialog closes.
  await waitFor(() =>
    expect(document.activeElement).toBe(
      screen.getByRole("button", { name: "More actions" }),
    ),
  );
});

test("an error is an alert, the region has no Alt+T in its name, and Alt+T moves nothing (A11Y-02, A11Y-04)", async () => {
  render(<Toaster />);
  act(() => {
    notify.error("The database is locked.");
  });
  const alert = await screen.findByText("The database is locked.");
  expect(alert.getAttribute("role")).toBe("alert");
  expect(screen.getByRole("region", { name: "Notifications" })).toBeTruthy();
  const before = document.activeElement;
  fireEvent.keyDown(document, { key: "t", code: "KeyT", altKey: true });
  expect(document.activeElement).toBe(before);
});

test("a failed Finish's toast sits bottom centre, clear of Finish, with its action under the sentence (A11Y-02)", async () => {
  render(<Toaster />);
  act(() => {
    notify.error("That focus is no longer offered.", {
      label: "Open the app anyway",
      onClick: () => {},
    });
  });
  // By its text: Sonner keeps an earlier test's error toast.
  const toast = (
    await screen.findByText("That focus is no longer offered.")
  ).closest("[data-sonner-toast]") as HTMLElement;
  const list = toast.closest("[data-sonner-toaster]") as HTMLElement;
  expect(list.dataset.xPosition).toBe("center");
  expect(list.dataset.yPosition).toBe("bottom");
  expect(toast.className).toContain("has-[[data-button]]:grid");
  const action = screen.getByRole("button", { name: "Open the app anyway" });
  expect(action.className).toContain("col-start-2");
});

test("a toast with an action stays until it's closed, so it can't close under the keyboard (A11Y-03)", async () => {
  vi.useFakeTimers();
  try {
    render(<Toaster />);
    act(() => {
      notify.success("Backup saved", {
        label: "Open folder",
        onClick: () => {},
      });
    });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(60_000);
    });
    const toast = screen
      .getByText("Backup saved")
      .closest("[data-sonner-toast]") as HTMLElement;
    expect(toast.dataset.removed).toBe("false");
    expect(toast.querySelector("[data-close-button]")).toBeTruthy();
  } finally {
    vi.useRealTimers();
  }
});
