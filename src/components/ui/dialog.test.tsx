import { act, fireEvent, render, screen } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { Button } from "./button";
import { ConfirmDialog, Dialog, DialogContent, DialogFooter } from "./dialog";
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
