import { CircleAlert, CircleCheck, TriangleAlert } from "lucide-react";
import { Toaster as Sonner, toast } from "sonner";

type Action = { label: string; onClick: () => void };

/** Rendered once. Toasts are Level 2 glass at the bottom right and close after 4 s (DESIGN_SYSTEM §9.1). */
export function Toaster() {
  return (
    <Sonner
      position="bottom-right"
      duration={4000}
      // One above the other, not piled up, so no toast hides another: a goal-moved note comes
      // just before a form's "Saved" (GOL-7, DESIGN_SYSTEM §9.1).
      expand
      // No Alt+T: it moved focus to the invisible list, and quick-add's Alt+T is Transfer (A11Y-04).
      hotkey={[]}
      customAriaLabel="Notifications"
      // Sonner's own stylesheet sets a system font, and it outranks Tailwind's classes.
      style={{ fontFamily: "inherit" }}
      icons={{
        success: <CircleCheck className="size-4 text-success" />,
        error: <CircleAlert className="size-4 text-danger" />,
        warning: <TriangleAlert className="size-4 text-warning" />,
      }}
      toastOptions={{
        unstyled: true,
        classNames: {
          // With an action, a grid: the action goes under the sentence, so the sentence keeps its
          // width, and ✕ (first in the DOM) sits top right, so the Tab order follows the layout.
          toast:
            "glass-raised flex w-[356px] items-center gap-2.5 rounded-toolbar py-2.5 pr-3 pl-3.5 text-body text-foreground has-[[data-button]]:grid has-[[data-button]]:grid-cols-[auto_1fr_auto] has-[[data-button]]:gap-y-1",
          content: "min-w-0 flex-1",
          // A ghost `sm` button (§9.1), its text in line with the sentence's.
          actionButton:
            "col-start-2 -ml-2.5 h-8 shrink-0 justify-self-start rounded-chip px-2.5 text-[13px] font-semibold text-primary hover:bg-wash",
          closeButton:
            "order-last col-start-3 row-start-1 grid size-6 shrink-0 place-items-center rounded-chip text-muted-foreground hover:bg-wash hover:text-foreground [&_svg]:size-3.5",
        },
      }}
    />
  );
}

export const notify = {
  /**
   * With an `action` (e.g. Open folder), it stays until closed, so it can't close while a keyboard
   * user is on its button (A11Y-03).
   */
  success: (message: string, action?: Action) =>
    toast.success(
      message,
      action && { action, duration: Infinity, closeButton: true },
    ),
  /** No icon, one line per `\n`, and 8 s, as a note can run to several lines. */
  info: (message: string) =>
    toast(message, { duration: 8_000, className: "whitespace-pre-line" }),
  /** Something to know about a save that went ahead, such as a card spending level (CC-11). */
  warning: (message: string) => toast.warning(message, { duration: 8_000 }),
  // Errors stay until closed, so they can't be missed, and are read out at once. Only a failed
  // Finish offers a way on (ADR 0014): bottom centre, between the check-in's Back and Finish, so it
  // never covers Finish (A11Y-02).
  error: (message: string, action?: Action) =>
    toast.error(<span role="alert">{message}</span>, {
      duration: Infinity,
      closeButton: true,
      action,
      ...(action && { position: "bottom-center" as const }),
    }),
  /** Closes a toast by the id `error` returned; none does nothing. */
  dismiss: (id: string | number | undefined) =>
    id !== undefined && toast.dismiss(id),
};
